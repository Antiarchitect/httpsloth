use std::io::{self, Write};
use std::sync::Arc;

use futures::Future;
use tokio::net::{tcp::ConnectFuture, TcpStream};
use tokio_rustls::{client::TlsStream, rustls::ClientConfig, webpki::DNSNameRef, TlsConnector};

pub enum MaybeHttpsStream {
    Http(Box<TcpStream>),
    Https(Box<TlsStream<TcpStream>>),
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
            let mut config = ClientConfig::new();
            config
                .root_store
                .add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
            let tls_connector = TlsConnector::from(Arc::new(config));

            Box::new(move |socket: ConnectFuture| -> BoxedMaybeHttps {
                let tls_connector = tls_connector.clone();
                let host = host.clone();
                Box::new(
                    socket
                        .and_then(move |socket| {
                            let dnsname = DNSNameRef::try_from_ascii_str(&host).unwrap();
                            tls_connector.connect(dnsname, socket).map_err(|e| {
                                io::Error::new(
                                    io::ErrorKind::Other,
                                    format!("TLS connector error: {}", e),
                                )
                            })
                        })
                        .map(|s| MaybeHttpsStream::Https(Box::new(s))),
                )
            })
        }
        "http" => Box::new(move |socket: ConnectFuture| -> BoxedMaybeHttps {
            Box::new(socket.map(|s| MaybeHttpsStream::Http(Box::new(s))))
        }),
        _scheme => panic!("Parsed URL scheme is not HTTP/HTTPS: {}", _scheme),
    }
}

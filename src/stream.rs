use std::io::{self, Write};

use super::tokio::net::TcpStream;
use super::tokio_tls::TlsStream;

pub enum MaybeHttpsStream {
    Http(TcpStream),
    Https(TlsStream<TcpStream>),
}

use MaybeHttpsStream::*;

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

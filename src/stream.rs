use std::io::{self, Read, Write};

use super::tokio_tls::TlsStream;
use super::tokio_core::net::TcpStream;


pub enum MaybeHttpsStream {
    Http(TcpStream),
    Https(TlsStream<TcpStream>),
}

use MaybeHttpsStream::*;

impl Read for MaybeHttpsStream {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Http(ref mut stream) => stream.read(buf),
            Https(ref mut stream) => stream.read(buf),
        }
    }
}

impl Write for MaybeHttpsStream {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Http(ref mut s) => s.write(buf),
            Https(ref mut s) => s.write(buf),
        }
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        match self {
            Http(ref mut s) => s.flush(),
            Https(ref mut s) => s.flush(),
        }
    }
}

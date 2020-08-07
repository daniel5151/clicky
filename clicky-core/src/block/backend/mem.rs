use std::io::{self, Cursor, Read, Seek, Write};
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::io::{AsyncRead, AsyncSeek, AsyncWrite};

use crate::block::BlockDev;

/// Memory-backed block device. No fancy features, just raw 1:1 access to an
/// in-memory buffer.
pub struct Mem {
    len: usize,
    data: Cursor<Box<[u8]>>,
}

impl std::fmt::Debug for Mem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mem")
            .field("len", &self.len)
            .field("data", &"[...]")
            .finish()
    }
}

impl Mem {
    /// Create a memory-backed block device from some existing data.
    pub fn new(data: Box<[u8]>) -> Mem {
        Mem {
            len: data.len(),
            data: Cursor::new(data),
        }
    }
}

impl BlockDev for Mem {
    fn len(&self) -> u64 {
        self.len as u64
    }
}

impl Read for Mem {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.data.read(buf)
    }
}

impl Write for Mem {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.data.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.data.flush()
    }
}

impl Seek for Mem {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.data.seek(pos)
    }
}

impl AsyncRead for Mem {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(io::Read::read(&mut *self, buf))
    }
}

impl AsyncWrite for Mem {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(io::Write::write(&mut *self, buf))
    }

    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(io::Write::flush(&mut *self))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

impl AsyncSeek for Mem {
    fn poll_seek(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        pos: io::SeekFrom,
    ) -> Poll<io::Result<u64>> {
        Poll::Ready(io::Seek::seek(&mut *self, pos))
    }
}

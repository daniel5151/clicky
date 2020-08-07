use std::fs::File;
use std::io::{self, Read, Seek, Write};
use std::pin::Pin;
use std::task::{Context, Poll};

use blocking::Unblock;
use futures::io::{AsyncRead, AsyncSeek, AsyncWrite};
use futures::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

use crate::block::BlockDev;

/// Raw, file-backed block device. No fancy features, just raw 1:1 access to
/// the underlying file's contents.
#[derive(Debug)]
pub struct Raw {
    len: u64,
    file: Unblock<File>,
}

impl Raw {
    pub fn new(file: File) -> io::Result<Raw> {
        Ok(Raw {
            len: file.metadata()?.len(),
            file: Unblock::new(file),
        })
    }
}

impl BlockDev for Raw {
    fn len(&self) -> u64 {
        self.len
    }
}

impl Read for Raw {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        futures_executor::block_on(async { AsyncReadExt::read(self, buf).await })
    }
}

impl Write for Raw {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        futures_executor::block_on(async { AsyncWriteExt::write(self, buf).await })
    }

    fn flush(&mut self) -> io::Result<()> {
        futures_executor::block_on(async { AsyncWriteExt::flush(self).await })
    }
}

impl Seek for Raw {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        futures_executor::block_on(async { AsyncSeekExt::seek(self, pos).await })
    }
}

impl AsyncRead for Raw {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        AsyncRead::poll_read(Pin::new(&mut self.file), cx, buf)
    }
}

impl AsyncWrite for Raw {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        AsyncWrite::poll_write(Pin::new(&mut self.file), cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        AsyncWrite::poll_flush(Pin::new(&mut self.file), cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        AsyncWrite::poll_close(Pin::new(&mut self.file), cx)
    }
}

impl AsyncSeek for Raw {
    fn poll_seek(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        pos: io::SeekFrom,
    ) -> Poll<io::Result<u64>> {
        AsyncSeek::poll_seek(Pin::new(&mut self.file), cx, pos)
    }
}

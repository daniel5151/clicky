use std::io::{self, Read, Seek, Write};

use crate::block::BlockDev;

/// Null block device. Can be configured to report any size, where reads always
/// return zero, and writes are a noop.
#[derive(Debug)]
pub struct Null {
    len: u64,
    offset: u64,
}

impl Null {
    pub fn new(reported_len: u64) -> Null {
        Null {
            len: reported_len,
            offset: 0,
        }
    }
}

impl BlockDev for Null {
    fn len(&self) -> io::Result<u64> {
        Ok(self.len)
    }
}

impl Read for Null {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        buf.iter_mut().for_each(|b| *b = 0);
        Ok(buf.len())
    }
}

impl Write for Null {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // noop
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        // noop
        Ok(())
    }
}

impl Seek for Null {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        // noop
        self.offset = match pos {
            io::SeekFrom::Start(v) => v,
            io::SeekFrom::End(v) => match self.len as i64 - v {
                o if o >= 0 => o as u64,
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "cannot seek to negative offset",
                    ))
                }
            },
            io::SeekFrom::Current(v) => match self.offset as i64 + v {
                o if o >= 0 => o as u64,
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "cannot seek to negative offset",
                    ))
                }
            },
        };

        Ok(self.offset)
    }
}

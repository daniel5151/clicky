use std::fs::File;
use std::io::{self, Read, Seek, Write};

use super::BlockDev;

/// Raw, file-backed block device. No fancy features, just raw 1:1 access to
/// the underlying file's contents.
#[derive(Debug)]
pub struct Raw {
    file: File,
}

impl Raw {
    pub fn new(file: File) -> Raw {
        Raw { file }
    }
}

impl BlockDev for Raw {
    fn len(&self) -> io::Result<u64> {
        Ok(self.file.metadata()?.len())
    }
}

impl Read for Raw {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }
}

impl Write for Raw {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

impl Seek for Raw {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.file.seek(pos)
    }
}

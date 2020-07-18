use std::io::{self, Cursor, Read, Seek, Write};

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
    fn len(&self) -> io::Result<u64> {
        Ok(self.len as u64)
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

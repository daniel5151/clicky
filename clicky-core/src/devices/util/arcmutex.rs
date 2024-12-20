use crate::devices::prelude::*;

use std::sync::Arc;
use std::sync::{LockResult, Mutex, MutexGuard};

/// Wrapper around Arc<Mutex<_>> which implements the `Device` and `Memory`
/// traits without having to explicitly deref + lock the underlying device.
#[derive(Debug)]
pub struct ArcMutexDevice<D> {
    device: Arc<Mutex<D>>,
}

impl<D> Clone for ArcMutexDevice<D> {
    fn clone(&self) -> Self {
        ArcMutexDevice {
            device: Arc::clone(&self.device),
        }
    }
}

impl<D> ArcMutexDevice<D> {
    /// Wrap the provided device in an Arc<Mutex<_>>
    pub fn new(device: D) -> ArcMutexDevice<D> {
        ArcMutexDevice {
            device: Arc::new(Mutex::new(device)),
        }
    }

    /// Lock the underlying device
    pub fn lock(&self) -> LockResult<MutexGuard<D>> {
        self.device.lock()
    }
}

impl<D: Device> Device for ArcMutexDevice<D> {
    fn kind(&self) -> &'static str {
        self.device.lock().unwrap().kind()
    }

    fn label(&self) -> Option<&'static str> {
        self.device.lock().unwrap().label()
    }

    fn probe(&self, offset: u32) -> Probe {
        self.device.lock().unwrap().probe(offset)
    }
}

impl<D: Memory> Memory for ArcMutexDevice<D> {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        self.device.lock().unwrap().r32(offset)
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        self.device.lock().unwrap().w32(offset, val)
    }

    fn r8(&mut self, offset: u32) -> MemResult<u8> {
        self.device.lock().unwrap().r8(offset)
    }

    fn r16(&mut self, offset: u32) -> MemResult<u16> {
        self.device.lock().unwrap().r16(offset)
    }

    fn w8(&mut self, offset: u32, val: u8) -> MemResult<()> {
        self.device.lock().unwrap().w8(offset, val)
    }

    fn w16(&mut self, offset: u32, val: u16) -> MemResult<()> {
        self.device.lock().unwrap().w16(offset, val)
    }

    fn x16(&mut self, offset: u32) -> MemResult<u16> {
        self.device.lock().unwrap().x16(offset)
    }

    fn x32(&mut self, offset: u32) -> MemResult<u32> {
        self.device.lock().unwrap().x32(offset)
    }
}

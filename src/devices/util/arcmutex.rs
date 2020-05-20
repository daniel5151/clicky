use std::sync::Arc;
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::devices::{Device, Probe};
use crate::memory::{MemResult, Memory};

/// Wrapper around Arc<Mutex<impl Device + Memory>>
#[derive(Debug)]
pub struct ArcMutexDevice<D> {
    label: Option<String>,
    pub device: Arc<Mutex<D>>,
}

impl<D> Clone for ArcMutexDevice<D> {
    fn clone(&self) -> Self {
        ArcMutexDevice {
            label: self.label.clone(),
            device: Arc::clone(&self.device),
        }
    }
}

impl<D: Device> ArcMutexDevice<D> {
    pub fn new(device: D) -> ArcMutexDevice<D> {
        ArcMutexDevice {
            label: device.label().map(|s| s.to_owned()),
            device: Arc::new(Mutex::new(device)),
        }
    }

    pub fn lock(&self) -> Result<MutexGuard<'_, D>, PoisonError<MutexGuard<'_, D>>> {
        self.device.lock()
    }
}

impl<D: Device> Device for ArcMutexDevice<D> {
    fn kind(&self) -> &'static str {
        self.device.lock().unwrap().kind()
    }

    fn label(&self) -> Option<&str> {
        self.label.as_ref().map(|s| s.as_str())
    }

    fn probe(&self, _offset: u32) -> Probe<'_> {
        Probe::Register("unimplemented")
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
}

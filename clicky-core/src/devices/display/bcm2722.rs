use crate::devices::prelude::*;
use crate::gui::RenderCallback;
use crate::devices::display::LcdPanel;

const DATA: u32 = 0x00000;
const WRITE_ADDR: u32 = 0x10000;
const READ_ADDR: u32 = 0x20000;
const STATUS: u32 = 0x30000;
const READY: u32 = 0x60000;
const HANDSHAKE: u32 = 0x70000;

#[derive(Debug)]
pub struct Bcm2722 {
    write_addr_latch: Option<u16>,
    read_latch: Option<u16>,
    read_latch_hi: Option<u32>,

    panel: Box<dyn LcdPanel>,
}

impl Bcm2722 {
    pub fn new(panel: Box<dyn LcdPanel>) -> Bcm2722 {
        Bcm2722 {
            write_addr_latch: None,
            read_latch: None,
            read_latch_hi: None,
            panel,
        }
    }

    pub fn render_callback(&self) -> RenderCallback {
        self.panel.render_callback()
    }
}

impl Device for Bcm2722 {
    fn kind(&self) -> &'static str {
        "BCM2722 Bridge"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            DATA => "Data",
            WRITE_ADDR => "Write Addr",
            READ_ADDR => "Read Addr",
            STATUS => "Status",
            READY => "Ready",
            HANDSHAKE => "Handshake",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for Bcm2722 {
    fn r16(&mut self, offset: u32) -> MemResult<u16> {
        if let Some(val) = self.read_latch.take() {
            return Ok(val as u16);
        }

        let val = match offset {
            DATA => {
                if let Some(val) = self.read_latch_hi.take() {
                    self.read_latch = Some((val >> 16) as u16); 
                    return Ok(val as u16);
                }
                let data32: u32 = match self.panel.read_data32() {
                    Ok(v) => v,
                    Err(_) => 0,
                };
                self.read_latch = Some((data32 >> 16) as u16); 
                data32 as u16
            }
            STATUS => 0x0013,
            WRITE_ADDR | READ_ADDR | 0x40000 | 0x50000 | READY => 0x0001,
            HANDSHAKE => 0x0040,
            _ => 0x0000,
        };
        Ok(val as u16)
    }

    fn r32(&mut self, _offset: u32) -> MemResult<u32> {
        Err(StubRead(Error, 0))
    }

    fn w16(&mut self, offset: u32, val: u16) -> MemResult<()> {
        let val = match self.write_addr_latch.take() {
            None => {
                self.write_addr_latch = Some(val);
                return Ok(());
            }
            Some(lo) => (lo as u32) | (val as u32) << 16,
        };

        self.w32(offset, val)
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            WRITE_ADDR => {
                let _ = self.panel.write_command32(val);
                Err(StubWrite(Trace, ()))
            }
            READ_ADDR => {
                match val {
                    0x01f8_0000 | 0x0c00_1000 => {
                        self.read_latch_hi = Some(1);
                        return Ok(());
                    }
                    _ => Err(StubWrite(Trace, ())),
                }
            }
            DATA | 4 | 0x40000 => {
                self.panel.write_data32(val)
            }
            _ => Err(StubWrite(Error, ())),
        }
    }
}

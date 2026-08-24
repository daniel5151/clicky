use crate::devices::prelude::*;

/// PP5020 serial controller
#[derive(Debug)]
pub struct Serial {
    label: &'static str,

    ier: u8,
    fcr: u8,
    lcr: u8,
    mcr: u8,
    asr: u8,
}

impl Serial {
    pub fn new(label: &'static str) -> Serial {
        Serial {
            label,

            ier: 0,
            fcr: 0,
            lcr: 0,
            mcr: 0,
            asr: 0,
        }
    }
}

impl Device for Serial {
    fn kind(&self) -> &'static str {
        "Serial"
    }

    fn label(&self) -> Option<&'static str> {
        Some(self.label)
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x00 => "RBR/THR",
            0x04 => "IER",
            0x08 => "FCR/IIR",
            0x0c => "LCR",
            0x10 => "MCR",
            0x14 => "LSR",
            0x18 => "MSR", // Modem Status Register
            0x1c => "SPR", // Scratch Pad Register
            0x20 => "IRDA", // IrDA Pulse Coding Register
            0x3c => "ASR", // Autobaud Sense Register
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for Serial {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x00 => {
                // TODO: properly wire up uart
                Err(StubRead(Info, 0))
            }
            0x04 => Ok(self.ier as u32),
            0x08 => Ok(self.fcr as u32),
            0x0c => Ok(self.lcr as u32),
            0x10 => Ok(self.mcr as u32),
            0x14 => Ok({
                let mut val : u8 = 0;
                val.set_bit(5, true) // TMTY / Transmit Shift Reg Empty status (1 = empty)
                   .set_bit(6, true); // THRE / Transmit Holding Register Empty (1 = empty)
                val as u32
            }),
            0x18..0x28 => Err(Unimplemented),
            0x3c => Ok(self.asr as u32),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        let val = val.trunc_to_u8()?;

        match offset {
            0x0 => Ok({
                // TODO: properly wire up uart
                if val.is_ascii() {
                    print!("{}", val as char);
                } else {
                    print!("\\x{:02x}", val);
                }
            }),
            0x04 => Ok(self.ier = val),
            0x08 => Ok(self.fcr = val),
            0x0c => Ok(self.lcr = val),
            0x10 => Ok(self.mcr = val),
            0x14 => Err(InvalidAccess),
            0x18..0x28 => Err(Unimplemented),
            0x3c => Ok(self.asr = val),
            _ => Err(Unexpected),
        }
    }
}

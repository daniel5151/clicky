use crate::devices::prelude::*;

mod pcf5060x;

pub mod i2c_devices {
    use super::*;

    pub use pcf5060x::*;
}

// TODO: move i2c devices + traits into separate folder
pub trait I2CDevice: Device {
    fn read(&mut self) -> MemResult<u8>;
    fn write(&mut self, data: u8) -> MemResult<()>;
}

impl Device for Box<dyn I2CDevice> {
    fn kind(&self) -> &'static str {
        (**self).kind()
    }

    fn label(&self) -> Option<&'static str> {
        (**self).label()
    }

    fn probe(&self, offset: u32) -> Probe {
        (**self).probe(offset)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum I2COp {
    Read,
    Write,
}

impl I2COp {
    fn into_bit(self) -> bool {
        match self {
            I2COp::Read => true,
            I2COp::Write => false,
        }
    }

    fn from_bit(bit: bool) -> Self {
        if bit {
            I2COp::Read
        } else {
            I2COp::Write
        }
    }
}

#[derive(Debug, Default, Copy, Clone)]
struct I2CTransactionCfg {
    // Set by writing to ADDR
    //
    // top 7 bits: address
    // lower byte: 1 = read, 0 = write
    addr_op: Option<I2COp>,
    addr: Option<u8>,
    // Set by writing to CONTROL
    //
    // bits 1..=2 - len
    // bit  5     - read = 1, write = 0 (?)
    // bit  7     - initiate transfer (SEND)
    op: Option<I2COp>,
    len: Option<u8>,
}

#[derive(Debug, Copy, Clone)]
struct I2CTransaction {
    op: I2COp,
    addr: u8,
    len: u8,
}

use crate::memory::{MemAccess, MemAccessKind, ToMemAccess};
use byteorder::{ByteOrder, LittleEndian};

impl I2CTransaction {
    fn to_memaccess(self, data: [u8; 4]) -> MemAccess {
        let addr = self.addr as u32; // FIXME: eew
        let kind = match self.op {
            I2COp::Read => MemAccessKind::Read,
            I2COp::Write => MemAccessKind::Write,
        };

        match self.len {
            0 => data[0].to_memaccess(addr, kind),
            1 => LittleEndian::read_u16(&data).to_memaccess(addr, kind),
            2 => LittleEndian::read_u24(&data).to_memaccess(addr, kind),
            3 => LittleEndian::read_u32(&data).to_memaccess(addr, kind),
            _ => unreachable!("invalid length"),
        }
    }
}

impl I2CTransactionCfg {
    fn take_txn(&mut self) -> MemResult<I2CTransaction> {
        // sanity check
        if self.addr_op != self.op {
            return Err(Fatal(
                "mismatch between op type in control (bit 5) and addr (bit 0)".into(),
            ));
        }

        let res = I2CTransaction {
            op: (self.op).ok_or_else(|| Fatal("did not specify op".into()))?,
            addr: (self.addr).ok_or_else(|| Fatal("did not specify addr".into()))?,
            len: (self.len).ok_or_else(|| Fatal("did not specify len".into()))?,
        };

        *self = I2CTransactionCfg::default();

        Ok(res)
    }
}

// XXX: so, Rust really doesn't like [Option<Box<dyn I2CDevice>>; 128].
// I mean, you'd think it'd be fine to just use [None; 128] to instantiate the
// array, but nooooo, you can't do that, because an option of a Box<dyn> doesn't
// implement Copy. Oof.
// This could be worked around with some unsafe, but in this case, I'm fine with
// just slapping together this little hack. Works like a charm.
mod hack {
    use super::*;

    use std::ops::{Index, IndexMut};

    #[derive(Default)]
    pub struct Devices {
        devices0: [Option<Box<dyn I2CDevice>>; 32],
        devices1: [Option<Box<dyn I2CDevice>>; 32],
        devices2: [Option<Box<dyn I2CDevice>>; 32],
        devices3: [Option<Box<dyn I2CDevice>>; 32],
    }

    impl Index<usize> for Devices {
        type Output = Option<Box<dyn I2CDevice>>;
        fn index(&self, idx: usize) -> &Option<Box<dyn I2CDevice>> {
            match idx {
                0..=31 => &self.devices0[idx],
                32..=63 => &self.devices1[idx - 32],
                64..=95 => &self.devices2[idx - 64],
                96..=127 => &self.devices3[idx - 96],
                _ => &self.devices3[idx - 96], // crashes with oob
            }
        }
    }

    impl IndexMut<usize> for Devices {
        fn index_mut(&mut self, idx: usize) -> &mut Option<Box<dyn I2CDevice>> {
            match idx {
                0..=31 => &mut self.devices0[idx],
                32..=63 => &mut self.devices1[idx - 32],
                64..=95 => &mut self.devices2[idx - 64],
                96..=127 => &mut self.devices3[idx - 96],
                _ => &mut self.devices3[idx - 96], // crashes with oob
            }
        }
    }
}

/// I2C Controller
pub struct I2CCon {
    devices: hack::Devices,

    busy: bool,
    txn: I2CTransactionCfg,
    data: [u8; 4],
}

impl std::fmt::Debug for I2CCon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("I2CCon")
            .field("devices", &"[...; 128]")
            .field("busy", &self.busy)
            .field("txn", &self.txn)
            .field("data", &self.data)
            .finish()
    }
}

impl I2CCon {
    pub fn new(_irq: irq::Sender) -> I2CCon {
        I2CCon {
            devices: hack::Devices::default(),

            busy: false,
            txn: I2CTransactionCfg::default(),
            data: [0; 4],
        }
    }

    /// Register a new i2c device with the controller, placing it at the given
    /// address. Returns any previously registered device at that address.
    ///
    /// Panics if `addr > 127`
    pub fn register_device(
        &mut self,
        addr: u8,
        device: Box<dyn I2CDevice>,
    ) -> Option<Box<dyn I2CDevice>> {
        assert!(addr < 128, "i2c addresses cannt be greater than 127!");
        std::mem::replace(&mut self.devices[addr as usize], Some(device))
    }

    // TODO?: This could be an async op (setting the busy flag + un-setting
    // when done)
    fn do_txn(&mut self, txn: I2CTransaction) -> MemResult<()> {
        let I2CTransaction { op, addr, len } = txn;

        let res = match self.devices[addr as usize] {
            Some(ref mut device) => {
                let mut err = Ok(());
                for b in self.data.iter_mut().take(len as usize + 1) {
                    // TODO: replace with try block once stabilized
                    let res = (|| match op {
                        I2COp::Read => match device.read() {
                            Ok(val) => Ok(*b = val),
                            Err(e) => {
                                match e {
                                    // If it's a stubbed-read, pass through the stubbed value
                                    StubRead(_, val)
                                    | ContractViolation {
                                        stub_val: Some(val),
                                        ..
                                    } => *b = val as _,
                                    _ => *b = 0x00, // arbitrary
                                }
                                Err(e)
                            }
                        },
                        I2COp::Write => Ok(device.write(*b)?),
                    })();
                    if let Err(e) = res {
                        err = Err(e);
                    }
                }
                err
            }
            None => Err(match op {
                I2COp::Read => StubRead(Debug, 0),
                I2COp::Write => StubWrite(Debug, ()),
            }),
        };
        trace!(
            target: "I2C",
            "i2c txn: {:02x?} {:02x?}",
            txn,
            &self.data[..len as usize + 1]
        );
        res
    }
}

impl Device for I2CCon {
    fn kind(&self) -> &'static str {
        "I2CCon"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x00 => "Control",
            0x04 => "Addr",
            0x0c => "Data0",
            0x10 => "Data1",
            0x14 => "Data2",
            0x18 => "Data3",
            0x1c => "Status",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for I2CCon {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x00 => Err(StubRead(Trace, {
                *0u8.set_bits(1..=2, self.txn.len.unwrap_or(0))
                    .set_bit(5, self.txn.op.map(|op| op.into_bit()).unwrap_or(false))
                    as u32
            })),
            0x04 => Ok({
                *0u8.set_bit(0, self.txn.addr_op.map(|op| op.into_bit()).unwrap_or(false))
                    .set_bits(1..=7, self.txn.addr.unwrap_or(0)) as u32
            }),
            0x0c => Ok(self.data[0] as u32),
            0x10 => Ok(self.data[1] as u32),
            0x14 => Ok(self.data[2] as u32),
            0x18 => Ok(self.data[3] as u32),
            0x1c => {
                // jiggle the busy status bit
                self.busy = !self.busy;
                Ok((self.busy as u32) << 6)
            }
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        let val = val.trunc_to_u8()?;

        match offset {
            0x00 => Err(StubWrite(Trace, {
                self.txn.len = Some(val.get_bits(1..=2));
                self.txn.op = Some(I2COp::from_bit(val.get_bit(5)));
                if val.get_bit(7) {
                    let txn = self.txn.take_txn()?;
                    if let Err(e) = self.do_txn(txn) {
                        // add i2c context
                        return Err(I2CException {
                            e: Box::new(e),
                            access: txn.to_memaccess(self.data),
                            in_device: match self.devices[txn.addr as usize] {
                                Some(ref device) => Probe::from_device(device, 0).to_string(),
                                None => "<unmapped i2c device>".into(),
                            },
                        });
                    }
                }
            })),
            0x04 => Ok({
                self.txn.addr_op = Some(I2COp::from_bit(val.get_bit(0)));
                self.txn.addr = Some(val.get_bits(1..=7));
            }),
            0x0c => Ok(self.data[0] = val),
            0x10 => Ok(self.data[1] = val),
            0x14 => Ok(self.data[2] = val),
            0x18 => Ok(self.data[3] = val),
            0x1c => Err(Unimplemented),
            _ => Err(Unexpected),
        }
    }
}

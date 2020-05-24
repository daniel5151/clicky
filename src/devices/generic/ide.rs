use bit_field::BitField;
use log::Level::*;

use crate::memory::{MemException::*, MemResult};

/// IDE Register to access.
///
/// LBAx registers are aliases for their corresponding CHS registers
/// (e.g: Using `IdeReg::CylinderLo` is equivalent to using `IdeReg::LBA0`).
///
/// Registers which share an address are aliases for one another (e.g: calling
/// `read(IdeReg::Features)` is equivalent to calling `read(IdeReg::Error)`.
#[derive(Debug)]
pub enum IdeReg {
    Data,
    Error,
    Features,
    SectorCount,
    SectorNo,
    CylinderLo,
    CylinderHi,
    DeviceHead,
    Status,
    Command,
    AltStatus,
    DevControl,
    DataLatch,
    Lba0,
    Lba1,
    Lba2,
    Lba3,
}

#[derive(Debug)]
struct IdeRegs {
    data: u16,
    error: u8,
    feature: u8,
    sector_count: u8,
    lba0_sector_no: u8,
    lba1_cyl_lo: u8,
    lba2_cyl_hi: u8,
    lba3_dev_head: u8,
    status: u8,
    command: u8,
    dev_control: u8,
}

#[derive(Debug)]
struct IdeDrive {
    reg: IdeRegs,
    // ...
}

/// Generic IDE Controller. Doesn't implement `Device` or `Memory` directly,
/// leaving those details up to the platform-specific implementations.
#[derive(Debug)]
pub struct IdeController {
    eightbit: bool,
    selected_device: bool, // u1
    ide0: Option<IdeDrive>,
    ide1: Option<IdeDrive>,
}

impl IdeController {
    pub fn new() -> IdeController {
        IdeController {
            eightbit: false,
            selected_device: false,
            ide0: None,
            ide1: None,
        }
    }

    fn current_dev(&mut self) -> MemResult<&mut IdeDrive> {
        match self.selected_device {
            false => self.ide0.as_mut(),
            true => self.ide1.as_mut(),
        }
        // not a real error. The OS might just be probing for IDE devices.
        .ok_or(ContractViolation {
            msg: format!(
                "tried to access IDE{} when no drive is connected",
                self.selected_device as u8
            ),
            severity: Info,
            stub_val: None,
        })
    }

    /// Perform a 16-bit read from an IDE register.
    ///
    /// NOTE: This method respects the current data-transfer size configuration
    /// of the IDE device. If the IDE device is running in 8-bit mode, this
    /// method will return the appropriate byte, albeit cast to a u16.
    pub fn read16(&mut self, reg: IdeReg) -> MemResult<u16> {
        match reg {
            IdeReg::Data if !self.eightbit => {
                // TODO
                Err(Unimplemented)
            }
            _ => self.read(reg).map(|v| v as u16),
        }
    }

    /// Perform a 16-bit write to an IDE register.
    ///
    /// NOTE: This method respects the current data-transfer size configuration
    /// of the IDE device. If the IDE device is running in 8-bit mode, this
    /// method will return the appropriate byte, albeit cast to a u16.
    pub fn write16(&mut self, reg: IdeReg, val: u16) -> MemResult<()> {
        match reg {
            IdeReg::Data if !self.eightbit => {
                // TODO
                Err(Unimplemented)
            }
            _ => self.write(reg, val as u8),
        }
    }

    /// Read a byte from an IDE register.
    pub fn read(&mut self, reg: IdeReg) -> MemResult<u8> {
        use IdeReg::*;

        let ide = self.current_dev()?;

        match reg {
            Data => Err(Unimplemented),
            Error | Features => Ok(ide.reg.error),
            SectorCount => Ok(ide.reg.sector_count),
            SectorNo | Lba0 => Ok(ide.reg.lba0_sector_no),
            CylinderLo | Lba1 => Ok(ide.reg.lba1_cyl_lo),
            CylinderHi | Lba2 => Ok(ide.reg.lba2_cyl_hi),
            DeviceHead | Lba3 => Ok(ide.reg.lba3_dev_head),
            Status | Command => {
                // TODO: ack interrupt
                Err(StubRead(Warn, ide.reg.status.into()))
            }
            AltStatus | DevControl => Ok(ide.reg.status),
            DataLatch => Err(Unimplemented),
        }
    }

    /// Write a byte to an IDE register.
    pub fn write(&mut self, reg: IdeReg, val: u8) -> MemResult<()> {
        use IdeReg::*;

        // set-up a convenient alias to the currently selected IDE device
        let ide = match reg {
            DeviceHead | Lba3 => {
                // FIXME?: Actually strip-out reserved bits?
                self.selected_device = val.get_bit(4); // DEV bit
                let ide = self.current_dev()?;
                return Ok(ide.reg.lba3_dev_head = val);
            }
            _ => self.current_dev()?,
        };

        match reg {
            Data => Err(Unimplemented),
            Features | Error => Ok(ide.reg.feature = val),
            SectorCount => Ok(ide.reg.sector_count = val),
            SectorNo | Lba0 => Ok(ide.reg.lba0_sector_no = val),
            CylinderLo | Lba1 => Ok(ide.reg.lba1_cyl_lo = val),
            CylinderHi | Lba2 => Ok(ide.reg.lba2_cyl_hi = val),
            DeviceHead | Lba3 => unreachable!("should have been handled above"),
            Command | Status => Err(Unimplemented),
            DevControl | AltStatus => Err(Unimplemented),
            DataLatch => Err(Unimplemented),
        }
    }
}

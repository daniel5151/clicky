use crate::devices::prelude::*;

use std::convert::TryFrom;
use std::io::{self, Read, Seek};

use num_enum::TryFromPrimitive;

use crate::block::BlockDev;
use crate::signal::irq;

// TODO?: make num heads / num sectors configurable?
const NUM_HEADS: usize = 16;
const NUM_SECTORS: usize = 63;

mod identify;
mod reg;

/// IDE Device (either 0 or 1)
#[derive(Debug, Copy, Clone)]
pub enum IdeIdx {
    IDE0,
    IDE1,
}

impl From<bool> for IdeIdx {
    fn from(b: bool) -> IdeIdx {
        match b {
            false => IdeIdx::IDE0,
            true => IdeIdx::IDE1,
        }
    }
}

impl std::fmt::Display for IdeIdx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdeIdx::IDE0 => write!(f, "IDE0"),
            IdeIdx::IDE1 => write!(f, "IDE1"),
        }
    }
}

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

#[derive(Debug, Default)]
struct IdeRegs {
    error: u8,
    feature: u8,
    sector_count: u8,
    lba0_sector_no: u8,
    lba1_cyl_lo: u8,
    lba2_cyl_hi: u8,
    lba3_dev_head: u8,
    status: u8,

    // Device Control
    /// software reset
    srst: bool,
    /// irq disabled
    nein: bool,
}

#[derive(Debug, PartialEq, Eq, TryFromPrimitive)]
#[repr(u8)]
enum IdeCmd {
    IdentifyDevice = 0xec,
    ReadMultiple = 0xc4,
    ReadSectors = 0x20,
    ReadSectorsNoRetry = 0x21,
    StandbyImmediate = 0xe0,
    StandbyImmediateAlt = 0x94,
    WriteSectors = 0x30,
    WriteSectorsNoRetry = 0x31,
    SetMultipleMode = 0xc6,

    // not strictly ATA-2, but the iPod flash ROM seems to use this cmd?
    FlushCache = 0xe7,
}

// TODO: provide a zero-copy constructor which uses the `Read` trait
struct IdeIoBuf {
    buf: [u8; 512],
    idx: usize,
}

impl IdeIoBuf {
    fn empty() -> IdeIoBuf {
        IdeIoBuf {
            buf: [0; 512],
            idx: 0,
        }
    }
}

impl std::fmt::Debug for IdeIoBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("IdeIoBuf")
            .field("buf", &"[...]")
            .field("idx", &self.idx)
            .finish()
    }
}

#[derive(Debug)]
enum IdeDriveState {
    Idle,
    Read {
        remaining_sectors: usize,
        iobuf: IdeIoBuf,
    },
    Write {
        remaining_sectors: usize,
        iobuf: IdeIoBuf,
    },
}

#[derive(Debug)]
struct IdeDrive {
    state: IdeDriveState,
    reg: IdeRegs,
    blockdev: Box<dyn BlockDev>,
    irq: irq::Sender, // shared between both drives

    eightbit: bool, // TODO: update this based on SetFeatures (0xef) command
    multi_sect: u8,
}

impl IdeDrive {
    /// Handles LBA/CHS offset translation, returning the offset into the
    /// blockdev (in blocks, _not bytes_).
    ///
    /// Returns `None` when the drive is in CHS mode but the registers contain
    /// invalid cyl/head/sector vals.
    fn get_sector_offset(&self) -> Option<u64> {
        let offset = if self.reg.lba3_dev_head.get_bit(reg::DEVHEAD::L) {
            (self.reg.lba3_dev_head.get_bits(reg::DEVHEAD::HS) as u64) << 24
                | (self.reg.lba2_cyl_hi as u64) << 16
                | (self.reg.lba1_cyl_lo as u64) << 8
                | (self.reg.lba0_sector_no as u64)
        } else {
            let sector = self.reg.lba0_sector_no as u64;
            let cyl = ((self.reg.lba2_cyl_hi as u16) << 8 | (self.reg.lba1_cyl_lo as u16)) as u64;
            let head = self.reg.lba3_dev_head.get_bits(reg::DEVHEAD::HS) as u64;

            // XXX: this should be pre-calculated, likely during the call to `attach()`
            let total_cyls = self.blockdev.len().unwrap() / (NUM_HEADS * NUM_SECTORS * 512) as u64;

            if sector < NUM_SECTORS as _ || cyl < total_cyls || head < NUM_HEADS as _ {
                return None;
            }

            ((cyl * NUM_HEADS as u64 + head) * NUM_SECTORS as u64 + sector) as u64
        };

        Some(offset)
    }

    fn data_read8(&mut self) -> MemResult<u8> {
        let (remaining_sectors, iobuf) = match self.state {
            IdeDriveState::Read {
                ref mut remaining_sectors,
                ref mut iobuf,
            } => (remaining_sectors, iobuf),
            _ => {
                // FIXME: this should set some error bits
                return Err(Fatal(format!(
                    "cannot read data while drive is in an invalid state: {:?}",
                    self.state
                )));
            }
        };

        // check if the next sector needs to be loaded first
        if iobuf.idx >= 512 {
            assert!(*remaining_sectors != 0);

            (self.reg.status)
                .set_bit(reg::STATUS::DRQ, false)
                .set_bit(reg::STATUS::BSY, true);

            // TODO: async this!
            iobuf.idx = 0;
            if let Err(e) = self.blockdev.read_exact(&mut iobuf.buf) {
                // XXX: actually set error bits
                return Err(e.into());
            }

            (self.reg.status)
                .set_bit(reg::STATUS::DRQ, true)
                .set_bit(reg::STATUS::BSY, false);

            // TODO: fire IRQ
        }

        let ret = iobuf.buf[iobuf.idx];
        iobuf.idx += 1;

        // check if there are no more sectors remaining
        if iobuf.idx >= 512 {
            *remaining_sectors -= 1; // FIXME: this varies under `ReadMultiple`
            if *remaining_sectors == 0 {
                self.state = IdeDriveState::Idle;
                (self.reg.status)
                    .set_bit(reg::STATUS::DRDY, true)
                    .set_bit(reg::STATUS::DRQ, false)
                    .set_bit(reg::STATUS::BSY, false);
                // TODO: fire IRQ
            }
        }

        Ok(ret)
    }

    fn data_write8(&mut self, val: u8) -> MemResult<()> {
        let (remaining_sectors, iobuf) = match self.state {
            IdeDriveState::Write {
                ref mut remaining_sectors,
                ref mut iobuf,
            } => (remaining_sectors, iobuf),
            _ => {
                // FIXME: this should set some error bits
                return Err(Fatal(format!(
                    "cannot write data while drive is in an invalid state: {:?}",
                    self.state
                )));
            }
        };

        iobuf.buf[iobuf.idx] = val;
        iobuf.idx += 1;

        // check if the sector needs to be flushed to disk
        if iobuf.idx >= 512 {
            assert!(*remaining_sectors != 0);

            (self.reg.status)
                .set_bit(reg::STATUS::DRQ, false)
                .set_bit(reg::STATUS::BSY, true);

            // TODO: async this!
            iobuf.idx = 0;
            if let Err(e) = self.blockdev.write_all(&iobuf.buf) {
                // XXX: actually set error bits
                return Err(e.into());
            }

            (self.reg.status)
                .set_bit(reg::STATUS::DRQ, true)
                .set_bit(reg::STATUS::BSY, false);

            // TODO: fire IRQ

            // check if there are no more sectors remaining
            *remaining_sectors -= 1; // FIXME: this varies under `WriteMultiple`
            if *remaining_sectors == 0 {
                self.state = IdeDriveState::Idle;
                (self.reg.status)
                    .set_bit(reg::STATUS::DRDY, true)
                    .set_bit(reg::STATUS::DRQ, false)
                    .set_bit(reg::STATUS::BSY, false);
                // TODO: fire IRQ
            }
        }

        Ok(())
    }

    fn exec_cmd(&mut self, cmd: u8) -> MemResult<()> {
        // TODO?: handle unsupported IDE command according to ATA spec
        let cmd = IdeCmd::try_from(cmd).map_err(|_| ContractViolation {
            msg: format!("unknown IDE command: {:#04x?}", cmd),
            severity: Error, // TODO: this should be Warn, and IDE error bits should be set
            stub_val: None,
        })?;

        (self.reg.status)
            .set_bit(reg::STATUS::ERR, false)
            .set_bit(reg::STATUS::BSY, true);
        self.reg.error = 0;

        use IdeCmd::*;
        match cmd {
            IdentifyDevice => {
                let len = self.blockdev.len()?;

                // fill the iobuf with identification info
                let drive_meta = identify::IdeDriveMeta {
                    total_sectors: len / 512,
                    cylinders: (len / (NUM_HEADS * NUM_SECTORS * 512) as u64) as u16,
                    heads: NUM_HEADS as u16,     // ?
                    sectors: NUM_SECTORS as u16, // ?
                    // TODO: generate these strings from blockdev info
                    serial: b"serials_are_4_chumps",
                    fw_version: b"0",
                    model: b"clickydrive",
                };

                // won't panic, since `hd_driveid` is statically asserted to be
                // exactly 512 bytes long.
                let mut iobuf = IdeIoBuf::empty();
                (iobuf.buf).copy_from_slice(bytemuck::bytes_of(&drive_meta.to_hd_driveid()));
                self.state = IdeDriveState::Read {
                    remaining_sectors: 1,
                    iobuf,
                };

                (self.reg.status)
                    .set_bit(reg::STATUS::BSY, false)
                    .set_bit(reg::STATUS::DRQ, true);

                // TODO: fire interrupt

                Ok(())
            }
            ReadMultiple => {
                if self.multi_sect == 0 {
                    // TODO?: use the ATA abort mechanism instead of loudly failing
                    return Err(ContractViolation {
                        msg: "Called ReadMultiple before successful call to SetMultipleMode".into(),
                        severity: Error,
                        stub_val: None,
                    });
                }

                if self.multi_sect != 1 {
                    Err(Fatal("(stubbed Multi-Sector support) cannot ReadMultiple (0xc4) with multi_sect > 1".into()))
                } else {
                    self.exec_cmd(ReadSectors as u8)
                }
            }
            ReadSectors | ReadSectorsNoRetry => {
                let offset = match self.get_sector_offset() {
                    Some(offset) => offset,
                    None => {
                        // XXX: actually set error bits
                        return Err(Fatal("invalid offset".into()));
                    }
                };

                // Seek into the blockdev
                if let Err(e) = self.blockdev.seek(io::SeekFrom::Start(offset * 512)) {
                    // XXX: actually set error bits
                    return Err(e.into());
                }

                // Read the first sector from the blockdev
                // TODO: this should be done asynchronously, with a separate task/thread
                // notifying the IDE device when the read is completed.
                let mut iobuf = IdeIoBuf::empty();
                if let Err(e) = self.blockdev.read_exact(&mut iobuf.buf) {
                    // XXX: actually set error bits
                    return Err(e.into());
                }

                self.state = IdeDriveState::Read {
                    remaining_sectors: if self.reg.sector_count == 0 {
                        256
                    } else {
                        self.reg.sector_count as usize
                    },
                    iobuf,
                };

                (self.reg.status)
                    .set_bit(reg::STATUS::BSY, false)
                    .set_bit(reg::STATUS::DSC, true)
                    .set_bit(reg::STATUS::DRDY, true)
                    .set_bit(reg::STATUS::DRQ, true);

                // TODO: fire interrupt

                Ok(())
            }
            StandbyImmediate | StandbyImmediateAlt => {
                // I mean, it's a virtual disk, there is no "spin up / spin down"
                self.reg.status.set_bit(reg::STATUS::BSY, false);

                // TODO: fire interrupt
                Ok(())
            }
            WriteSectors | WriteSectorsNoRetry => {
                // NOTE: this code is somewhat UNTESTED

                let offset = match self.get_sector_offset() {
                    Some(offset) => offset,
                    None => {
                        // XXX: actually set error bits
                        return Err(Fatal("invalid offset".into()));
                    }
                };

                // Seek into the blockdev
                if let Err(e) = self.blockdev.seek(io::SeekFrom::Start(offset * 512)) {
                    // XXX: actually set error bits
                    return Err(e.into());
                }

                self.state = IdeDriveState::Write {
                    remaining_sectors: if self.reg.sector_count == 0 {
                        256
                    } else {
                        self.reg.sector_count as usize
                    },
                    iobuf: IdeIoBuf::empty(),
                };

                (self.reg.status)
                    .set_bit(reg::STATUS::BSY, false)
                    .set_bit(reg::STATUS::DSC, true)
                    .set_bit(reg::STATUS::DRDY, false)
                    .set_bit(reg::STATUS::DRQ, true);

                // TODO: fire interrupt?

                Ok(())
            }

            SetMultipleMode => {
                self.multi_sect = self.reg.sector_count;

                // TODO: implement proper multi-sector support
                if self.multi_sect > 1 {
                    return Err(Fatal (
                        "(stubbed Multi-Sector support) SetMultipleMode (0xc6) must be either 0 or 1".into(),
                    ));
                }

                (self.reg.status).set_bit(reg::STATUS::BSY, false);
                Ok(())
            }

            FlushCache => {
                // uhh, we don't implement caching
                (self.reg.status)
                    .set_bit(reg::STATUS::BSY, false)
                    .set_bit(reg::STATUS::DRDY, true)
                    .set_bit(reg::STATUS::DRQ, false);

                Ok(())
            }
        }
    }
}

/// Generic IDE Controller. Doesn't implement `Device` or `Memory` directly, as
/// those vary between platform-specific implementations.
#[derive(Debug)]
pub struct IdeController {
    common_irq_line: irq::Sender,
    selected_device: IdeIdx,
    ide0: Option<IdeDrive>,
    ide1: Option<IdeDrive>,
}

/// It'd be nice if this was a method, but it makes borrowing other fields of
/// (&mut self) as pain, since the borrow checker doesn't work across function
/// boundaries.
///
/// Returns MemResult<&mut IdeDrive>
macro_rules! selected_ide {
    ($self:ident) => {
        match $self.selected_device {
            IdeIdx::IDE0 => $self.ide0.as_mut(),
            IdeIdx::IDE1 => $self.ide1.as_mut(),
        }
        // not a real error. The OS might just be probing for IDE devices.
        .ok_or(ContractViolation {
            msg: format!(
                "tried to access {} when no drive is connected",
                $self.selected_device
            ),
            severity: Info,
            stub_val: Some(0xff), // OSDev Wiki recommends 0xff as "open bus" val
        })
    };
}

impl IdeController {
    pub fn new(irq: irq::Sender) -> IdeController {
        IdeController {
            common_irq_line: irq,
            selected_device: IdeIdx::IDE0,
            ide0: None,
            ide1: None,
        }
    }

    /// Attach a block device to the IDE controller. Returns the
    /// previously-attached block device (if applicable).
    pub fn attach(
        &mut self,
        idx: IdeIdx,
        blockdev: Box<dyn BlockDev>,
    ) -> Option<Box<dyn BlockDev>> {
        let old_drive = self.detach(idx);

        let ide = match idx {
            IdeIdx::IDE0 => &mut self.ide0,
            IdeIdx::IDE1 => &mut self.ide1,
        };

        let new_drive = IdeDrive {
            state: IdeDriveState::Idle,
            reg: IdeRegs {
                status: *0u8.set_bit(reg::STATUS::DRDY, true),
                ..IdeRegs::default()
            },
            blockdev,
            irq: self.common_irq_line.clone(),

            eightbit: false,
            multi_sect: 0,
        };

        *ide = Some(new_drive);

        old_drive
    }

    /// Detaches a block device from the IDE drive. Returns the
    /// previously-attached block device (if applicable).
    pub fn detach(&mut self, idx: IdeIdx) -> Option<Box<dyn BlockDev>> {
        let ide = match idx {
            IdeIdx::IDE0 => &mut self.ide0,
            IdeIdx::IDE1 => &mut self.ide1,
        };

        ide.take().map(|ide| ide.blockdev)
    }

    /// Check if an IDE drive is currently asserting an IRQ.
    pub fn irq_state(&self, idx: IdeIdx) -> bool {
        let ide = match idx {
            IdeIdx::IDE0 => &self.ide0,
            IdeIdx::IDE1 => &self.ide1,
        };

        ide.as_ref()
            .map(|ide| ide.irq.is_asserting())
            .unwrap_or(false)
    }

    /// Clears an IDE drive's IRQ.
    pub fn clear_irq(&mut self, idx: IdeIdx) {
        let ide = match idx {
            IdeIdx::IDE0 => &mut self.ide0,
            IdeIdx::IDE1 => &mut self.ide1,
        };

        if let Some(ide) = ide.as_mut() {
            ide.irq.clear()
        }
    }

    /// Perform a 16-bit read from an IDE register.
    ///
    /// NOTE: This method respects the current data-transfer size configuration
    /// of the IDE device. If the IDE device is running in 8-bit mode, this
    /// method will return the appropriate byte, albeit cast to a u16.
    pub fn read16(&mut self, reg: IdeReg) -> MemResult<u16> {
        match reg {
            IdeReg::Data => {
                let ide = selected_ide!(self)?;
                let val = ide.data_read8()?;
                if ide.eightbit {
                    Ok(val as u16)
                } else {
                    let hi_val = ide.data_read8()?;
                    Ok(val as u16 | (hi_val as u16) << 8)
                }
            }
            _ => self.read8(reg).map(|v| v as u16),
        }
    }

    /// Perform a 16-bit write to an IDE register.
    ///
    /// NOTE: This method respects the current data-transfer size configuration
    /// of the IDE device. If the IDE device is running in 8-bit mode, this
    /// method will return the appropriate byte, albeit cast to a u16.
    pub fn write16(&mut self, reg: IdeReg, val: u16) -> MemResult<()> {
        match reg {
            IdeReg::Data => {
                let ide = selected_ide!(self)?;

                if ide.eightbit {
                    ide.data_write8(val as u8)?;
                } else {
                    ide.data_write8(val as u8)?;
                    ide.data_write8((val >> 8) as u8)?;
                }

                Ok(())
            }
            _ => self.write8(reg, val as u8),
        }
    }

    /// Read a byte from an IDE register.
    pub fn read8(&mut self, reg: IdeReg) -> MemResult<u8> {
        use IdeReg::*;

        let ide = selected_ide!(self)?;

        match reg {
            Data => ide.data_read8(),
            Error | Features => Ok(ide.reg.error),
            SectorCount => Ok(ide.reg.sector_count),
            SectorNo | Lba0 => Ok(ide.reg.lba0_sector_no),
            CylinderLo | Lba1 => Ok(ide.reg.lba1_cyl_lo),
            CylinderHi | Lba2 => Ok(ide.reg.lba2_cyl_hi),
            DeviceHead | Lba3 => Ok(ide.reg.lba3_dev_head),
            Status | Command => {
                ide.irq.clear(); // ack IRQ
                Ok(ide.reg.status)
            }
            AltStatus | DevControl => Ok(ide.reg.status),
            DataLatch => Err(Unimplemented),
        }
    }

    /// Write a byte to an IDE register.
    pub fn write8(&mut self, reg: IdeReg, val: u8) -> MemResult<()> {
        use IdeReg::*;

        // set-up a convenient alias to the currently selected IDE device
        let ide = match reg {
            DeviceHead | Lba3 => {
                // FIXME?: Actually strip-out reserved bits?
                self.selected_device = val.get_bit(reg::DEVHEAD::DEV).into();
                let ide = selected_ide!(self)?;
                return Ok(ide.reg.lba3_dev_head = val);
            }
            _ => selected_ide!(self)?,
        };

        match reg {
            Data => ide.data_write8(val as u8),
            Features | Error => Ok(ide.reg.feature = val),
            SectorCount => Ok(ide.reg.sector_count = val),
            SectorNo | Lba0 => Ok(ide.reg.lba0_sector_no = val),
            CylinderLo | Lba1 => Ok(ide.reg.lba1_cyl_lo = val),
            CylinderHi | Lba2 => Ok(ide.reg.lba2_cyl_hi = val),
            DeviceHead | Lba3 => unreachable!("should be handled above"),
            Command | Status => ide.exec_cmd(val),
            DevControl | AltStatus => {
                ide.reg.srst = val.get_bit(2);
                ide.reg.nein = val.get_bit(1);
                Ok(())
            }
            DataLatch => Err(Unimplemented),
        }
    }
}

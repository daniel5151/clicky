use std::convert::TryFrom;
use std::io::{self, Read, Seek, Write};

use bit_field::BitField;
use log::Level::*;
use num_enum::TryFromPrimitive;

use crate::block::BlockDev;
use crate::memory::{MemException::*, MemResult};

// TODO?: make num heads / num sectors configurable?
const NUM_HEADS: usize = 16;
const NUM_SECTORS: usize = 63;

mod identify;

/// IDE status register bits
#[allow(non_snake_case, unused)]
mod STATUS {
    /// Busy
    pub const BSY: usize = 7;
    /// Device Ready
    pub const DRDY: usize = 6;
    /// Device Fault
    pub const DF: usize = 5;
    /// Disk Seek Complete
    pub const DSC: usize = 4;
    /// Data Request
    pub const DRQ: usize = 3;
    /// Corrected Data
    pub const CORR: usize = 2;
    /// Index (vendor specific)
    pub const IDX: usize = 1;
    /// Error
    pub const ERR: usize = 0;
}

#[allow(non_snake_case, unused)]
mod DEVHEAD {
    type Range = std::ops::RangeInclusive<usize>;
    /// LBA addressing
    pub const L: usize = 6;
    /// Device Index
    pub const DEV: usize = 4;
    /// Bits 24..=27 of the LBA address
    pub const HS: Range = 0..=3;
}

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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
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
    Standby = 0xe0,
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
    Write,
}

#[derive(Debug)]
struct IdeDrive {
    state: IdeDriveState,
    eightbit: bool, // FIXME?: I think this can be derived from reg.features?
    reg: IdeRegs,
    blockdev: Box<dyn BlockDev>,
}

impl IdeDrive {
    /// Handles LBA/CHS offset translation, returning the offset into the
    /// blockdev (in blocks, _not bytes_).
    ///
    /// Returns `None` when the drive is in CHS mode but the registers contain
    /// invalid cyl/head/sector vals.
    fn get_sector_offset(&self) -> Option<u64> {
        let offset = if self.reg.lba3_dev_head.get_bit(DEVHEAD::L) {
            (self.reg.lba3_dev_head.get_bits(DEVHEAD::HS) as u64) << 24
                | (self.reg.lba2_cyl_hi as u64) << 16
                | (self.reg.lba1_cyl_lo as u64) << 8
                | (self.reg.lba0_sector_no as u64)
        } else {
            let sector = self.reg.lba0_sector_no as u64;
            let cyl = ((self.reg.lba2_cyl_hi as u16) << 8 | (self.reg.lba1_cyl_lo as u16)) as u64;
            let head = self.reg.lba3_dev_head.get_bits(DEVHEAD::HS) as u64;

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
                return Err(FatalError(format!(
                    "cannot read data while drive is in an invalid state: {:?}",
                    self.state
                )));
            }
        };

        // check if the next sector needs to be loaded first
        if iobuf.idx >= 512 {
            assert!(*remaining_sectors != 0);

            (self.reg.status)
                .set_bit(STATUS::DRQ, false)
                .set_bit(STATUS::BSY, true);

            iobuf.idx = 0;
            if let Err(e) = self.blockdev.read_exact(&mut iobuf.buf) {
                // XXX: actually set error bits
                return Err(e)?;
            }

            (self.reg.status)
                .set_bit(STATUS::DRQ, true)
                .set_bit(STATUS::BSY, false);

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
                    .set_bit(STATUS::DRDY, true)
                    .set_bit(STATUS::DRQ, false)
                    .set_bit(STATUS::BSY, false);
                // TODO: fire IRQ
            }
        }

        Ok(ret)
    }

    fn data_write8(&mut self, _val: u8) -> MemResult<()> {
        Err(Unimplemented)
    }

    fn exec_cmd(&mut self, cmd: u8) -> MemResult<()> {
        // TODO?: handle unsupported IDE command according to ATA spec
        let cmd = IdeCmd::try_from(cmd).map_err(|_| ContractViolation {
            msg: format!("unknown IDE command: {:#04x?}", cmd),
            severity: Error, // TODO: this should be Warn, and IDE error bits should be set
            stub_val: None,
        })?;

        macro_rules! unimplemented_cmd {
            () => {
                return Err(FatalError(format!("unimplemented IDE command: {:?}", cmd)));
            };
        }

        (self.reg.status)
            .set_bit(STATUS::ERR, false)
            .set_bit(STATUS::BSY, true);
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
                    .set_bit(STATUS::BSY, false)
                    .set_bit(STATUS::DRQ, true);

                // TODO: fire interrupt

                Ok(())
            }
            ReadMultiple => unimplemented_cmd!(),
            ReadSectors | ReadSectorsNoRetry => {
                let offset = match self.get_sector_offset() {
                    Some(offset) => offset,
                    None => {
                        // XXX: actually set error bits
                        return Err(FatalError("invalid offset".into()));
                    }
                };

                // Seek into the blockdev
                if let Err(e) = self.blockdev.seek(io::SeekFrom::Start(offset * 512)) {
                    // XXX: actually set error bits
                    return Err(e)?;
                }

                // Read the first sector from the blockdev
                let mut iobuf = IdeIoBuf::empty();
                if let Err(e) = self.blockdev.read_exact(&mut iobuf.buf) {
                    // XXX: actually set error bits
                    return Err(e)?;
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
                    .set_bit(STATUS::BSY, false)
                    .set_bit(STATUS::DSC, true)
                    .set_bit(STATUS::DRDY, true)
                    .set_bit(STATUS::DRQ, true);

                // TODO: fire interrupt

                Ok(())
            }
            Standby => unimplemented_cmd!(),
        }
    }
}

/// Generic IDE Controller. Doesn't implement `Device` or `Memory` directly, as
/// those vary between platform-specific implementations.
#[derive(Debug)]
pub struct IdeController {
    selected_device: IdeIdx,
    ide0: Option<IdeDrive>,
    ide1: Option<IdeDrive>,
}

impl IdeController {
    pub fn new() -> IdeController {
        IdeController {
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
            eightbit: false,
            reg: IdeRegs {
                status: *0u8.set_bit(STATUS::DRDY, true),
                ..IdeRegs::default()
            },
            blockdev,
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

    fn current_dev(&mut self) -> MemResult<&mut IdeDrive> {
        match self.selected_device {
            IdeIdx::IDE0 => self.ide0.as_mut(),
            IdeIdx::IDE1 => self.ide1.as_mut(),
        }
        // not a real error. The OS might just be probing for IDE devices.
        .ok_or(ContractViolation {
            msg: format!(
                "tried to access {} when no drive is connected",
                self.selected_device
            ),
            severity: Info,
            stub_val: Some(0xff), // OSDev Wiki recommends 0xff as "open bus" val
        })
    }

    /// Perform a 16-bit read from an IDE register.
    ///
    /// NOTE: This method respects the current data-transfer size configuration
    /// of the IDE device. If the IDE device is running in 8-bit mode, this
    /// method will return the appropriate byte, albeit cast to a u16.
    pub fn read16(&mut self, reg: IdeReg) -> MemResult<u16> {
        match reg {
            IdeReg::Data => {
                let ide = self.current_dev()?;
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
                let ide = self.current_dev()?;
                ide.data_write8(val as u8)?;

                if !ide.eightbit {
                    ide.data_write8(val as u8)?;
                }

                Ok(())
            }
            _ => self.write8(reg, val as u8),
        }
    }

    /// Read a byte from an IDE register.
    pub fn read8(&mut self, reg: IdeReg) -> MemResult<u8> {
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
                Err(StubRead(Info, ide.reg.status.into()))
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
                self.selected_device = val.get_bit(DEVHEAD::DEV).into();
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

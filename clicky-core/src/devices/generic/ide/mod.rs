use crate::devices::prelude::*;

use std::convert::TryFrom;
use std::io;

use futures::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use num_enum::TryFromPrimitive;

use crate::block::BlockDev;

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

#[derive(Debug, PartialEq, Eq, TryFromPrimitive)]
#[repr(u8)]
enum IdeCmd {
    IdentifyDevice = 0xec,
    ReadMultiple = 0xc4,
    WriteMultiple = 0xc5,
    ReadSectors = 0x20,
    ReadSectorsNoRetry = 0x21,
    StandbyImmediate = 0xe0,
    StandbyImmediateAlt = 0x94,
    WriteSectors = 0x30,
    WriteSectorsNoRetry = 0x31,
    SetMultipleMode = 0xc6,
    SetFeatures = 0xef,
    ReadDMA = 0xc8,
    ReadDMANoRetry = 0xc9,
    WriteDMA = 0xca,
    WriteDMANoRetry = 0xcb,

    InitializeDriveParameters = 0x91,

    Sleep = 0x99,
    SleepAlt = 0xe6,

    // not strictly ATA-2, but the iPod flash ROM seems to use this cmd...
    FlushCache = 0xe7,
}

mod iobuf {
    // TODO: provide a zero-copy constructor which uses the `Read` trait
    pub struct IdeIoBuf {
        buf: [u8; 512],
        idx: usize,
    }

    impl std::fmt::Debug for IdeIoBuf {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
            f.debug_struct("IdeIoBuf")
                .field("buf", &"[...]")
                .field("idx", &self.idx)
                .finish()
        }
    }

    impl IdeIoBuf {
        pub fn empty() -> IdeIoBuf {
            IdeIoBuf {
                buf: [0; 512],
                idx: 0,
            }
        }

        pub fn read8(&mut self) -> Option<u8> {
            let ret = *self.buf.get(self.idx)?;
            self.idx += 1;
            Some(ret)
        }

        pub fn write8(&mut self, val: u8) -> Option<()> {
            *self.buf.get_mut(self.idx)? = val;
            self.idx += 1;
            Some(())
        }

        /// Reset internal cursor to start of buffer.
        pub fn new_transfer(&mut self) {
            self.idx = 0;
        }

        /// Checked if the transfer is done
        pub fn is_done_transfer(&self) -> bool {
            self.idx >= 512
        }

        pub fn as_raw(&mut self) -> &mut [u8; 512] {
            &mut self.buf
        }
    }
}
use iobuf::IdeIoBuf;

#[derive(Debug)]
enum IdeDriveState {
    Idle,
    ReadReady,
    ReadAsyncLoad,
    WriteReady,
    WriteAsyncFlush,
}

/// Transfer Mode set by the "Set Transfer Mode" (0x03) subcommand of the "Set
/// Features" command.
#[derive(Debug)]
enum IdeTransferMode {
    Pio,
    PioNoIORDY,
    PioFlowControl(u8),
    DMASingleWord(u8),
    DMAMultiWord(u8),
    Reserved,
    Invalid,
}

impl From<u8> for IdeTransferMode {
    fn from(val: u8) -> IdeTransferMode {
        IdeTransferMode::from_u8(val)
    }
}

impl IdeTransferMode {
    fn from_u8(val: u8) -> IdeTransferMode {
        use self::IdeTransferMode::*;

        match val.leading_zeros() {
            8 => Pio,
            7 => PioNoIORDY,
            6 => Invalid,
            5 => Invalid,
            4 => PioFlowControl(val & 7),
            3 => DMASingleWord(val & 7),
            2 => DMAMultiWord(val & 7),
            1 => Reserved,
            0 => Reserved,
            _ => unreachable!(),
        }
    }

    fn is_dma(&self) -> bool {
        use self::IdeTransferMode::*;
        matches!(self, DMASingleWord(..) | DMAMultiWord(..))
    }
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

/// Various IDE toggles and features.
#[derive(Debug)]
struct IdeDriveConfig {
    eightbit: bool,
    multi_sect: u8,
    // NOTE: from what I understand, the transfer mode is only used for hardware timings, and
    // doesn't actually impact emulation?
    transfer_mode: IdeTransferMode,
}

#[derive(Debug)]
struct IdeDrive {
    blockdev: Box<dyn BlockDev>,
    irq: irq::Sender,   // shared between both drives
    dmarq: irq::Sender, // shared between both drives

    state: IdeDriveState,
    remaining_sectors: usize,

    iobuf: IdeIoBuf,
    reg: IdeRegs,
    cfg: IdeDriveConfig,
}

impl IdeDrive {
    fn new(irq: irq::Sender, dmarq: irq::Sender, blockdev: Box<dyn BlockDev>) -> IdeDrive {
        IdeDrive {
            blockdev,
            irq,
            dmarq,

            state: IdeDriveState::Idle,
            remaining_sectors: 0,

            iobuf: IdeIoBuf::empty(),
            reg: IdeRegs {
                status: *0u8.set_bit(reg::STATUS::DRDY, true),
                ..IdeRegs::default()
            },
            cfg: IdeDriveConfig {
                eightbit: false,
                multi_sect: 0,
                transfer_mode: IdeTransferMode::Pio,
            },
        }
    }

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

            let total_cyls = self.blockdev.len() / (NUM_HEADS * NUM_SECTORS * 512) as u64;

            if sector < NUM_SECTORS as _ || cyl < total_cyls || head < NUM_HEADS as _ {
                return None;
            }

            ((cyl * NUM_HEADS as u64 + head) * NUM_SECTORS as u64 + sector) as u64
        };

        Some(offset)
    }

    fn data_read8(&mut self) -> MemResult<u8> {
        match self.state {
            IdeDriveState::ReadReady => {}
            _ => {
                // FIXME: this should set some error bits
                return Err(Fatal(format!(
                    "cannot read data while drive is in an invalid state: {:?}",
                    self.state
                )));
            }
        }

        let ret = self
            .iobuf
            .read8()
            .ok_or_else(|| Fatal("assert: read past end of IDE iobuf".into()))?;

        if self.iobuf.is_done_transfer() {
            // check if there are no more sectors remaining
            self.remaining_sectors -= 1; // TODO: this varies under `ReadMultiple`
            if self.remaining_sectors == 0 {
                self.state = IdeDriveState::Idle;
                (self.reg.status)
                    .set_bit(reg::STATUS::DRDY, true)
                    .set_bit(reg::STATUS::DRQ, false)
                    .set_bit(reg::STATUS::BSY, false);

                self.irq.assert();
                self.dmarq.clear();
            } else {
                // the next sector needs to be loaded
                (self.reg.status)
                    .set_bit(reg::STATUS::DRQ, false)
                    .set_bit(reg::STATUS::BSY, true);

                self.state = IdeDriveState::ReadAsyncLoad;

                futures_executor::block_on(async {
                    // TODO: async this!
                    if let Err(e) = self.blockdev.read_exact(self.iobuf.as_raw()).await {
                        // XXX: actually set error bits
                        return Err(e);
                    }

                    self.iobuf.new_transfer();
                    self.state = IdeDriveState::ReadReady;
                    (self.reg.status)
                        .set_bit(reg::STATUS::DRQ, true)
                        .set_bit(reg::STATUS::BSY, false);

                    // DMA only fires a single IRQ at the end of the transfer
                    if !self.cfg.transfer_mode.is_dma() {
                        self.irq.assert();
                    }

                    Ok(())
                })?;
            }
        }

        Ok(ret)
    }

    fn data_write8(&mut self, val: u8) -> MemResult<()> {
        match self.state {
            IdeDriveState::WriteReady => {}
            _ => {
                // FIXME: this should set some error bits
                return Err(Fatal(format!(
                    "cannot write data while drive is in an invalid state: {:?}",
                    self.state
                )));
            }
        }

        self.iobuf
            .write8(val)
            .ok_or_else(|| Fatal("assert: write past end of IDE iobuf".into()))?;

        // check if the sector needs to be flushed to disk
        if self.iobuf.is_done_transfer() {
            assert!(self.remaining_sectors != 0);

            (self.reg.status)
                .set_bit(reg::STATUS::DRQ, false)
                .set_bit(reg::STATUS::BSY, true);

            self.state = IdeDriveState::WriteAsyncFlush;

            // TODO: async this!
            futures_executor::block_on(async {
                if let Err(e) = self.blockdev.write_all(self.iobuf.as_raw()).await {
                    // XXX: actually set error bits
                    return Err(e);
                }

                self.iobuf.new_transfer();
                self.state = IdeDriveState::WriteReady;
                (self.reg.status)
                    .set_bit(reg::STATUS::DRQ, true)
                    .set_bit(reg::STATUS::BSY, false);

                // DMA only fires a single IRQ at the end of the transfer
                if !self.cfg.transfer_mode.is_dma() {
                    self.irq.assert();
                }

                // check if there are no more sectors remaining
                self.remaining_sectors -= 1; // FIXME: this varies under `WriteMultiple`
                if self.remaining_sectors == 0 {
                    self.state = IdeDriveState::Idle;
                    (self.reg.status)
                        .set_bit(reg::STATUS::DRDY, true)
                        .set_bit(reg::STATUS::DRQ, false);

                    self.irq.assert();
                    self.dmarq.clear();
                }

                Ok(())
            })?;
        }

        Ok(())
    }

    fn exec_cmd(&mut self, cmd: u8) -> MemResult<()> {
        if (self.reg.status).get_bit(reg::STATUS::BSY) {
            return Err(ContractViolation {
                msg: "tried to exec IDE cmd while drive is busy".into(),
                severity: Warn,
                stub_val: None,
            });
        }

        // TODO?: handle unsupported IDE command according to ATA spec
        let cmd = IdeCmd::try_from(cmd).map_err(|_| ContractViolation {
            msg: format!("unknown IDE command: {:#04x?}", cmd),
            severity: Error, // TODO: this should be Warn, and IDE error bits should be set
            stub_val: None,
        })?;

        (self.reg.status)
            .set_bit(reg::STATUS::BSY, true)
            .set_bit(reg::STATUS::ERR, false);
        self.reg.error = 0;

        use IdeCmd::*;
        match cmd {
            IdentifyDevice => {
                let len = self.blockdev.len();

                // fill the iobuf with identification info
                let drive_meta = identify::IdeDriveMeta {
                    total_sectors: len / 512,
                    cylinders: (len / (NUM_HEADS * NUM_SECTORS * 512) as u64) as u16,
                    heads: NUM_HEADS as u16,     // ?
                    sectors: NUM_SECTORS as u16, // ?
                    // TODO: generate these strings though blockdev interface?
                    serial: b"serials_are_4_chumps",
                    fw_version: b"0",
                    model: b"clickydrive",
                };

                // won't panic, since `hd_driveid` is statically asserted to be
                // exactly 512 bytes long.
                (self.iobuf.as_raw())
                    .copy_from_slice(bytemuck::bytes_of(&drive_meta.to_hd_driveid()));

                self.iobuf.new_transfer();
                self.state = IdeDriveState::ReadReady;
                self.remaining_sectors = 1;

                (self.reg.status)
                    .set_bit(reg::STATUS::BSY, false)
                    .set_bit(reg::STATUS::DRQ, true);

                self.irq.assert();

                Ok(())
            }
            ReadMultiple => {
                if self.cfg.multi_sect == 0 {
                    // TODO?: use the ATA abort mechanism instead of loudly failing
                    return Err(ContractViolation {
                        msg: "Called ReadMultiple before successful call to SetMultipleMode".into(),
                        severity: Error,
                        stub_val: None,
                    });
                }

                if self.cfg.multi_sect != 1 {
                    Err(Fatal("(stubbed Multi-Sector support) cannot ReadMultiple (0xc4) with multi_sect > 1".into()))
                } else {
                    (self.reg.status).set_bit(reg::STATUS::BSY, false);
                    self.exec_cmd(ReadSectors as u8)
                }
            }
            ReadDMA | ReadDMANoRetry => {
                if !self.cfg.transfer_mode.is_dma() {
                    // TODO?: use the ATA abort mechanism instead of loudly failing
                    return Err(ContractViolation {
                        msg: "Called ReadDMA without setting DMA transfer mode".into(),
                        severity: Error,
                        stub_val: None,
                    });
                }

                // basically just ReadSectors, except it only fires a _single_
                // IRQ at the end of the transfer, and asserts dmarq
                self.dmarq.assert();
                (self.reg.status).set_bit(reg::STATUS::BSY, false);
                self.exec_cmd(ReadSectors as u8)
            }
            ReadSectors | ReadSectorsNoRetry => {
                let offset = match self.get_sector_offset() {
                    Some(offset) => offset,
                    None => {
                        // XXX: actually set error bits
                        return Err(Fatal("invalid offset".into()));
                    }
                };

                self.state = IdeDriveState::ReadAsyncLoad;
                futures_executor::block_on(async {
                    // Seek into the blockdev
                    if let Err(e) = self.blockdev.seek(io::SeekFrom::Start(offset * 512)).await {
                        // XXX: actually set error bits
                        return Err(e);
                    }

                    // Read the first sector from the blockdev
                    // TODO: this should be done asynchronously, with a separate task/thread
                    // notifying the IDE device when the read is completed.
                    if let Err(e) = self.blockdev.read_exact(self.iobuf.as_raw()).await {
                        // XXX: actually set error bits
                        return Err(e);
                    }

                    self.remaining_sectors = if self.reg.sector_count == 0 {
                        256
                    } else {
                        self.reg.sector_count as usize
                    };

                    self.iobuf.new_transfer();
                    self.state = IdeDriveState::ReadReady;
                    (self.reg.status)
                        .set_bit(reg::STATUS::BSY, false)
                        .set_bit(reg::STATUS::DSC, true)
                        .set_bit(reg::STATUS::DRDY, true)
                        .set_bit(reg::STATUS::DRQ, true);

                    // TODO: fire interrupt?

                    Ok(())
                })?;

                Ok(())
            }
            StandbyImmediate | StandbyImmediateAlt => {
                // I mean, it's a virtual disk, there is no "spin up / spin down"
                self.reg.status.set_bit(reg::STATUS::BSY, false);

                // TODO: fire interrupt
                Ok(())
            }
            WriteMultiple => {
                if self.cfg.multi_sect == 0 {
                    // TODO?: use the ATA abort mechanism instead of loudly failing
                    return Err(ContractViolation {
                        msg: "Called WriteMultiple before successful call to SetMultipleMode"
                            .into(),
                        severity: Error,
                        stub_val: None,
                    });
                }

                if self.cfg.multi_sect != 1 {
                    Err(Fatal("(stubbed Multi-Sector support) cannot WriteMultiple (0xc4) with multi_sect > 1".into()))
                } else {
                    (self.reg.status).set_bit(reg::STATUS::BSY, false);
                    self.exec_cmd(WriteSectors as u8)
                }
            }
            WriteDMA | WriteDMANoRetry => {
                if !self.cfg.transfer_mode.is_dma() {
                    // TODO?: use the ATA abort mechanism instead of loudly failing
                    return Err(ContractViolation {
                        msg: "Called WriteDMA without setting DMA transfer mode".into(),
                        severity: Error,
                        stub_val: None,
                    });
                }

                // basically just WriteSectors, except it only fires a _single_
                // IRQ at the end of the transfer, and asserts dmarq
                self.dmarq.assert();
                (self.reg.status).set_bit(reg::STATUS::BSY, false);
                self.exec_cmd(WriteSectors as u8)
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

                self.state = IdeDriveState::WriteAsyncFlush;
                futures_executor::block_on(async {
                    // Seek into the blockdev
                    if let Err(e) = self.blockdev.seek(io::SeekFrom::Start(offset * 512)).await {
                        // XXX: actually set error bits
                        return Err(e);
                    }

                    self.remaining_sectors = if self.reg.sector_count == 0 {
                        256
                    } else {
                        self.reg.sector_count as usize
                    };

                    self.iobuf.new_transfer();
                    self.state = IdeDriveState::WriteReady;
                    (self.reg.status)
                        .set_bit(reg::STATUS::BSY, false)
                        .set_bit(reg::STATUS::DSC, true)
                        .set_bit(reg::STATUS::DRDY, false)
                        .set_bit(reg::STATUS::DRQ, true);

                    // TODO: fire interrupt?

                    Ok(())
                })?;

                Ok(())
            }

            SetMultipleMode => {
                self.cfg.multi_sect = self.reg.sector_count;

                // TODO: implement proper multi-sector support
                if self.cfg.multi_sect > 1 {
                    return Err(Fatal (
                        "(stubbed Multi-Sector support) SetMultipleMode (0xc6) must be either 0 or 1".into(),
                    ));
                }

                (self.reg.status).set_bit(reg::STATUS::BSY, false);
                Ok(())
            }

            SetFeatures => {
                match self.reg.feature {
                    // Enable 8-bit data transfers
                    0x01 => self.cfg.eightbit = true,
                    // Set transfer mode based on value in Sector Count register
                    0x03 => self.cfg.transfer_mode = IdeTransferMode::from(self.reg.sector_count),
                    // Disable 8-bit data transfers
                    0x81 => self.cfg.eightbit = false,
                    other => {
                        return Err(Fatal(format!(
                            "SetFeatures (0xef) subcommand not implemented: {:#04x?}",
                            other
                        )))
                    }
                };

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

            Sleep | SleepAlt => {
                // uhh, it's an emulated drive.
                // just assert the irq and go on our merry way
                (self.reg.status).set_bit(reg::STATUS::BSY, false);

                self.irq.assert();
                Ok(())
            }

            InitializeDriveParameters => {
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
    dmarq: irq::Sender,

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
    pub fn new(irq: irq::Sender, dmarq: irq::Sender) -> IdeController {
        IdeController {
            common_irq_line: irq,
            dmarq,
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

        *ide = Some(IdeDrive::new(
            self.common_irq_line.clone(),
            self.dmarq.clone(),
            blockdev,
        ));
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
                let ret = if ide.cfg.eightbit {
                    val as u16
                } else {
                    let hi_val = ide.data_read8()?;
                    val as u16 | (hi_val as u16) << 8
                };

                Ok(ret)
            }
            _ => self.read8(reg).map(|v| v as u16),
        }
    }

    /// Perform a 16-bit write to an IDE register.
    ///
    /// NOTE: This method respects the current data-transfer size configuration
    /// of the IDE device. If the IDE device is running in 8-bit mode but the
    /// value is >8 bits, a ContractViolation error will occur.
    pub fn write16(&mut self, reg: IdeReg, val: u16) -> MemResult<()> {
        match reg {
            IdeReg::Data => {
                let ide = selected_ide!(self)?;

                if ide.cfg.eightbit {
                    let val = val.trunc_to_u8()?;
                    ide.data_write8(val)?;
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
            Data => ide.data_write8(val),
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

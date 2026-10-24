use crate::devices::prelude::*;

use std::convert::TryFrom;
use std::io;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use futures::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use num_enum::TryFromPrimitive;

use crate::block::BlockDev;

// TODO?: make num heads / num sectors configurable?
const NUM_HEADS: usize = 16;
const NUM_SECTORS: usize = 63;

mod identify;
mod reg;
mod smart;

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
    ReadNativeMax = 0xf8,

    InitializeDriveParameters = 0x91,

    Smart = 0xb0,

    Sleep = 0x99,
    SleepAlt = 0xe6,

    // not strictly ATA-2, but the iPod flash ROM seems to use this cmd...
    FlushCache = 0xe7,
}

#[derive(Debug, PartialEq, Eq, TryFromPrimitive)]
#[repr(u8)]
enum IdeSetFeaturesReg {
    Enable8BitPIOTransferMode = 0x01,
    Disable8BitPIOTransferMode = 0x81,

    SetTransferModeBasedOnValueInSectorCountRegister = 0x03,

    EnableWriteCache = 0x02,
    DisableWriteCache = 0x82,

    EnableReadLookAhead = 0xAA,
    DisableReadLookAhead = 0x55,

    EnableRevertingToPowerOnDefaults = 0xcc,
    DisableRevertingToPowerOnDefaults = 0x66,

    EnableAdvancedPowerManagement = 0x05,
    DisableAdvancedPowerManagement = 0x85,

    EnableCFAPowerMode1 = 0x0A,
    DisableCFAPowerMode1 = 0x8A,

    EnableAutomaticAcousticManagement = 0x42,
    DisableAutomaticAcousticManagement = 0xC2,
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

/// How many system ticks the drive stays BSY between a sector's data phase and
/// the next one (or command completion).
const MEDIA_LATENCY_STEPS: u32 = 128;

#[derive(Debug, Clone, Copy)]
enum IdeDriveState {
    Idle,
    Busy { steps: u32, next: DataPhase },
    ReadReady,
    WriteReady,
}

#[derive(Debug, Clone, Copy)]
enum DataPhase {
    Read,
    Write,
    Done,
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

#[derive(Debug, Default, Copy, Clone)]
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
    task_spawner: Spawner,

    state: IdeDriveState,
    remaining_sectors: usize,
    dma: bool, // Command being processed is a DMA one

    iobuf: IdeIoBuf,
    reg: Arc<RwLock<IdeRegs>>,
    cfg: IdeDriveConfig,
}

impl IdeDrive {
    fn new(
        irq: irq::Sender,
        dmarq: irq::Sender,
        blockdev: Box<dyn BlockDev>,
        task_spawner: Spawner,
    ) -> IdeDrive {
        IdeDrive {
            blockdev,
            irq,
            dmarq,
            task_spawner,

            state: IdeDriveState::Idle,
            remaining_sectors: 0,
            dma: false,

            iobuf: IdeIoBuf::empty(),
            reg: Arc::new(RwLock::new(IdeRegs {
                status: *0u8.set_bit(reg::STATUS::DRDY, true),
                ..IdeRegs::default()
            })),
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
        let reg = self.reg.read().unwrap();

        let offset = if reg.lba3_dev_head.get_bit(reg::DEVHEAD::L) {
            (reg.lba3_dev_head.get_bits(reg::DEVHEAD::HS) as u64) << 24
                | (reg.lba2_cyl_hi as u64) << 16
                | (reg.lba1_cyl_lo as u64) << 8
                | (reg.lba0_sector_no as u64)
        } else {
            let sector = reg.lba0_sector_no as u64;
            let cyl = ((reg.lba2_cyl_hi as u16) << 8 | (reg.lba1_cyl_lo as u16)) as u64;
            let head = reg.lba3_dev_head.get_bits(reg::DEVHEAD::HS) as u64;

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
            self.remaining_sectors -= 1; // TODO: this varies under `ReadMultiple`
            if self.remaining_sectors == 0 {
                self.finish_command();
            } else {
                self.load_sector()?;
                self.go_busy(DataPhase::Read);
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

        if self.iobuf.is_done_transfer() {
            // XXX: actually set error bits
            futures_executor::block_on(self.blockdev.write_all(self.iobuf.as_raw()))?;

            self.remaining_sectors -= 1; // FIXME: this varies under `WriteMultiple`
            self.go_busy(match self.remaining_sectors {
                0 => DataPhase::Done,
                _ => DataPhase::Write,
            });
        }

        Ok(())
    }

    /// Advance the drive by one system step.
    fn tick(&mut self) {
        let next = match &mut self.state {
            IdeDriveState::Busy { steps: 0, next } => *next,
            IdeDriveState::Busy { steps, .. } => return *steps -= 1,
            _ => return,
        };

        match next {
            DataPhase::Read => self.enter_data_phase(IdeDriveState::ReadReady, !self.dma),
            DataPhase::Write => self.enter_data_phase(IdeDriveState::WriteReady, !self.dma),
            DataPhase::Done => self.finish_command(),
        }
    }

    fn load_sector(&mut self) -> MemResult<()> {
        // XXX: actually set error bits
        futures_executor::block_on(self.blockdev.read_exact(self.iobuf.as_raw()))?;
        Ok(())
    }

    /// Seek to the sector addressed by the taskfile and latch the transfer
    /// length.
    fn begin_transfer(&mut self, dma: bool) -> MemResult<()> {
        let offset = self
            .get_sector_offset()
            // XXX: actually set error bits
            .ok_or_else(|| Fatal("invalid offset".into()))?;
        futures_executor::block_on(self.blockdev.seek(io::SeekFrom::Start(offset * 512)))?;

        self.remaining_sectors = match self.reg.read().unwrap().sector_count {
            0 => 256,
            n => n as usize,
        };
        self.dma = dma;
        Ok(())
    }

    fn start_read(&mut self, dma: bool) -> MemResult<()> {
        self.begin_transfer(dma)?;
        self.load_sector()?;
        self.go_busy(DataPhase::Read);
        Ok(())
    }

    fn start_write(&mut self, dma: bool) -> MemResult<()> {
        self.begin_transfer(dma)?;
        // TODO: ATA requests the first data-out block without an interrupt
        self.enter_data_phase(IdeDriveState::WriteReady, true);
        Ok(())
    }

    fn go_busy(&mut self, next: DataPhase) {
        self.state = IdeDriveState::Busy {
            steps: MEDIA_LATENCY_STEPS,
            next,
        };
        (self.reg.write().unwrap().status)
            .set_bit(reg::STATUS::BSY, true)
            .set_bit(reg::STATUS::DRQ, false);
        self.dmarq.clear();
    }

    fn enter_data_phase(&mut self, state: IdeDriveState, interrupt: bool) {
        self.iobuf.new_transfer();
        self.state = state;
        (self.reg.write().unwrap().status)
            .set_bit(reg::STATUS::BSY, false)
            .set_bit(reg::STATUS::DSC, true)
            .set_bit(reg::STATUS::DRDY, true)
            .set_bit(reg::STATUS::DRQ, true);

        if self.dma {
            self.dmarq.assert();
        }
        if interrupt {
            self.assert_intrq();
        }
    }

    fn finish_command(&mut self) {
        self.state = IdeDriveState::Idle;
        (self.reg.write().unwrap().status)
            .set_bit(reg::STATUS::BSY, false)
            .set_bit(reg::STATUS::DRDY, true)
            .set_bit(reg::STATUS::DRQ, false);

        self.assert_intrq();
        self.dmarq.clear();
    }

    fn assert_intrq(&mut self) {
        if !self.reg.read().unwrap().nein {
            self.irq.assert()
        }
    }

    /// Finish the current command after `delay`, without blocking the CPU.
    fn complete_after(&self, delay: Duration) {
        self.task_spawner
            .spawn({
                let reg_arc = Arc::clone(&self.reg);
                let mut irq = self.irq.clone();
                let timer = relativity::Timeout::new(delay);
                async move {
                    timer.await;
                    let nein = {
                        let mut reg = reg_arc.write().unwrap();
                        (reg.status)
                            .set_bit(reg::STATUS::BSY, false)
                            .set_bit(reg::STATUS::DSC, true);
                        reg.nein
                    };
                    if !nein {
                        irq.assert();
                    }
                }
            })
            .expect("failed to spawn timer task");
    }

    fn exec_cmd(&mut self, cmd: u8) -> MemResult<()> {
        let reg = self.reg.read().unwrap().clone();
        if (reg.status).get_bit(reg::STATUS::BSY) {
            return Err(ContractViolation {
                msg: "tried to exec IDE cmd while drive is busy".into(),
                severity: Warn,
                stub_val: None,
            });
        }

        {
            let mut reg = self.reg.write().unwrap();
            reg.status = (1 << reg::STATUS::BSY | 1 << reg::STATUS::DRDY) as u8;
            reg.error = 0;
        }
        self.dma = false;

        if let Ok(cmd_val) = IdeCmd::try_from(cmd) {
            trace!(
                target: "IDE",
                "Executing command: {:?}",
                cmd_val,
            );
        }

        if cmd & 0xf0 == 0x10 {
            self.reg
                .write()
                .unwrap()
                .status
                .set_bit(reg::STATUS::BSY, false);
            self.assert_intrq();
            return Ok(());
        }

        use IdeCmd::*;
        match IdeCmd::try_from(cmd) {
            Ok(IdentifyDevice) => {
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

                {
                    let mut reg = self.reg.write().unwrap();
                    (reg.status)
                        .set_bit(reg::STATUS::BSY, false)
                        .set_bit(reg::STATUS::DRQ, true);
                }

                self.assert_intrq();

                Ok(())
            }
            Ok(ReadMultiple) => {
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
                    self.start_read(false)
                }
            }
            Ok(ReadDMA) | Ok(ReadDMANoRetry) => {
                if !self.cfg.transfer_mode.is_dma() {
                    // TODO?: use the ATA abort mechanism instead of loudly failing
                    return Err(ContractViolation {
                        msg: "Called ReadDMA without setting DMA transfer mode".into(),
                        severity: Error,
                        stub_val: None,
                    });
                }

                self.start_read(true)
            }
            Ok(ReadSectors) | Ok(ReadSectorsNoRetry) => self.start_read(false),
            Ok(StandbyImmediate) | Ok(StandbyImmediateAlt) => {
                // I mean, it's a virtual disk, there is no "spin up / spin down"
                {
                    let mut reg = self.reg.write().unwrap();
                    reg.status.set_bit(reg::STATUS::BSY, false);
                }

                self.assert_intrq();
                Ok(())
            }
            Ok(WriteMultiple) => {
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
                    self.start_write(false)
                }
            }
            Ok(WriteDMA) | Ok(WriteDMANoRetry) => {
                if !self.cfg.transfer_mode.is_dma() {
                    // TODO?: use the ATA abort mechanism instead of loudly failing
                    return Err(ContractViolation {
                        msg: "Called WriteDMA without setting DMA transfer mode".into(),
                        severity: Error,
                        stub_val: None,
                    });
                }

                self.start_write(true)
            }
            Ok(WriteSectors) | Ok(WriteSectorsNoRetry) => self.start_write(false),

            Ok(SetMultipleMode) => {
                self.cfg.multi_sect = reg.sector_count;

                // TODO: implement proper multi-sector support
                if self.cfg.multi_sect > 1 {
                    return Err(Fatal (
                        "(stubbed Multi-Sector support) SetMultipleMode (0xc6) must be either 0 or 1".into(),
                    ));
                }

                {
                    let mut reg = self.reg.write().unwrap();
                    (reg.status).set_bit(reg::STATUS::BSY, false);
                }
                Ok(())
            }

            Ok(SetFeatures) => {
                use IdeSetFeaturesReg::*;
                match IdeSetFeaturesReg::try_from(reg.feature) {
                    Ok(Enable8BitPIOTransferMode) => self.cfg.eightbit = true,
                    Ok(SetTransferModeBasedOnValueInSectorCountRegister) => {
                        self.cfg.transfer_mode = IdeTransferMode::from(reg.sector_count)
                    }
                    Ok(Disable8BitPIOTransferMode) => self.cfg.eightbit = false,

                    // Irrelevant on an emulated drive
                    Ok(EnableWriteCache)
                    | Ok(DisableWriteCache)
                    | Ok(EnableReadLookAhead)
                    | Ok(DisableReadLookAhead)
                    | Ok(EnableRevertingToPowerOnDefaults)
                    | Ok(DisableRevertingToPowerOnDefaults)
                    | Ok(EnableAdvancedPowerManagement)
                    | Ok(DisableAdvancedPowerManagement)
                    | Ok(EnableCFAPowerMode1)
                    | Ok(DisableCFAPowerMode1)
                    | Ok(EnableAutomaticAcousticManagement)
                    | Ok(DisableAutomaticAcousticManagement) => {}

                    other => {
                        return Err(Fatal(format!(
                            "SetFeatures (0xef) subcommand not implemented: {:#04x?}",
                            other
                        )))
                    }
                };

                {
                    let mut reg = self.reg.write().unwrap();
                    (reg.status)
                        .set_bit(reg::STATUS::BSY, false)
                        .set_bit(reg::STATUS::DRDY, true);
                }

                self.assert_intrq();
                Ok(())
            }

            Ok(FlushCache) => {
                // uhh, we don't implement caching
                let mut reg = self.reg.write().unwrap();
                (reg.status)
                    .set_bit(reg::STATUS::BSY, false)
                    .set_bit(reg::STATUS::DRDY, true)
                    .set_bit(reg::STATUS::DRQ, false);

                Ok(())
            }

            Ok(Sleep) | Ok(SleepAlt) => {
                // uhh, it's an emulated drive.
                // just assert the irq and go on our merry way
                self.reg
                    .write()
                    .unwrap()
                    .status
                    .set_bit(reg::STATUS::BSY, false);

                self.assert_intrq();
                Ok(())
            }

            Ok(InitializeDriveParameters) => {
                self.complete_after(Duration::from_millis(100));
                Ok(())
            }

            Ok(ReadNativeMax) => {
                self.complete_after(Duration::from_millis(100));
                Ok(())
            }

            Ok(Smart) => {
                // The SMART command set is keyed off a signature in the
                // cylinder registers -- without it the command must abort.
                if reg.lba1_cyl_lo != 0x4f || reg.lba2_cyl_hi != 0xc2 {
                    {
                        let mut reg = self.reg.write().unwrap();
                        (reg.status)
                            .set_bit(reg::STATUS::BSY, false)
                            .set_bit(reg::STATUS::DRDY, true)
                            .set_bit(reg::STATUS::ERR, true);
                        reg.error = *0u8.set_bit(reg::ERROR::ABRT, true);
                    }
                    self.assert_intrq();
                    return Err(ContractViolation {
                        msg: format!(
                            "SMART (0xb0) without the 0xc24f signature (cyl={:02x}{:02x})",
                            reg.lba2_cyl_hi, reg.lba1_cyl_lo
                        ),
                        severity: Warn,
                        stub_val: None,
                    });
                }

                // subcommand lives in the features register
                match reg.feature {
                    // ENABLE / DISABLE OPERATIONS, ENABLE / DISABLE ATTRIBUTE
                    // AUTOSAVE, EXECUTE OFF-LINE IMMEDIATE. There's no real
                    // SMART state to keep, so these just succeed.
                    0xd2 | 0xd4 | 0xd8 | 0xd9 => {
                        {
                            let mut reg = self.reg.write().unwrap();
                            (reg.status)
                                .set_bit(reg::STATUS::BSY, false)
                                .set_bit(reg::STATUS::DRDY, true);
                        }
                        self.assert_intrq();
                        Ok(())
                    }
                    // RETURN STATUS. The drive reports health back through the
                    // cylinder registers: the unchanged 0xc24f signature means
                    // "no threshold exceeded", i.e. a healthy drive.
                    0xda => {
                        {
                            let mut reg = self.reg.write().unwrap();
                            reg.lba1_cyl_lo = 0x4f;
                            reg.lba2_cyl_hi = 0xc2;
                            (reg.status)
                                .set_bit(reg::STATUS::BSY, false)
                                .set_bit(reg::STATUS::DRDY, true);
                        }
                        self.assert_intrq();
                        Ok(())
                    }
                    // READ DATA / READ ATTRIBUTE THRESHOLDS. Both hand back a
                    // 512 byte structure.
                    0xd0 | 0xd1 => {
                        use smart::*;
                        let mut smart = Smart::new();
                        const ATTRIBUTES: &[(AttributeId, u16, u8, u8, u64, u8)] = &[
                            (AttributeId::ReadErrorRate, 0x000f, 100, 100, 0, 51),
                            (AttributeId::SpinUpTime, 0x0027, 100, 100, 1200, 1),
                            (AttributeId::StartStopCount, 0x0032, 100, 100, 128, 0),
                            (AttributeId::ReallocatedSectorCount, 0x0033, 100, 100, 0, 5),
                            (AttributeId::SeekErrorRate, 0x000f, 100, 100, 0, 51),
                            (AttributeId::PowerOnHours, 0x0032, 100, 100, 42, 0),
                            (AttributeId::SpinRetryCount, 0x0013, 100, 100, 0, 51),
                            (AttributeId::PowerCycleCount, 0x0032, 100, 100, 64, 0),
                            (
                                AttributeId::UnexpectedPowerOffCount,
                                0x0032,
                                100,
                                100,
                                12,
                                0,
                            ),
                            (AttributeId::Temperature, 0x0022, 100, 100, 30, 0),
                        ];
                        for (id, flags, current, worst, raw, threshold) in ATTRIBUTES {
                            if let Err(e) = smart.add_attribute(Attribute {
                                id: *id as u8,
                                flags: *flags,
                                current: *current,
                                worst: *worst,
                                raw: *raw,
                                threshold: *threshold,
                            }) {
                                panic!("Error while adding attribute to SMART: {:?}", e);
                            }
                        }

                        match reg.feature {
                            0xd0 => {
                                self.iobuf.as_raw().copy_from_slice(&smart.serialize_data());
                            }
                            0xd1 => {
                                self.iobuf
                                    .as_raw()
                                    .copy_from_slice(&smart.serialize_threshold());
                            }
                            _ => unreachable!("unknown feature"),
                        }
                        self.iobuf.new_transfer();
                        self.state = IdeDriveState::ReadReady;
                        self.remaining_sectors = 1;

                        {
                            let mut reg = self.reg.write().unwrap();
                            (reg.status)
                                .set_bit(reg::STATUS::BSY, false)
                                .set_bit(reg::STATUS::DRDY, true)
                                .set_bit(reg::STATUS::DRQ, true);
                        }
                        self.assert_intrq();
                        Ok(())
                    }
                    other => {
                        {
                            let mut reg = self.reg.write().unwrap();
                            (reg.status)
                                .set_bit(reg::STATUS::BSY, false)
                                .set_bit(reg::STATUS::DRDY, true)
                                .set_bit(reg::STATUS::ERR, true);
                            reg.error = *0u8.set_bit(reg::ERROR::ABRT, true);
                        }
                        self.assert_intrq();
                        Err(ContractViolation {
                            msg: format!("unsupported SMART subcommand (aborted): {:#04x?}", other),
                            severity: Warn,
                            stub_val: None,
                        })
                    }
                }
            }

            Err(_) => {
                {
                    let mut reg = self.reg.write().unwrap();
                    (reg.status)
                        .set_bit(reg::STATUS::BSY, false)
                        .set_bit(reg::STATUS::DRDY, true)
                        .set_bit(reg::STATUS::DRQ, false)
                        .set_bit(reg::STATUS::ERR, true);
                    reg.error = *0u8.set_bit(reg::ERROR::ABRT, true);
                }

                self.assert_intrq();

                Err(ContractViolation {
                    msg: format!("unknown IDE command: {:#04x?}", cmd),
                    severity: Warn,
                    stub_val: None,
                })
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
    task_spawner: Spawner,

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
    pub fn new(irq: irq::Sender, dmarq: irq::Sender, task_spawner: Spawner) -> IdeController {
        IdeController {
            common_irq_line: irq,
            dmarq,
            selected_device: IdeIdx::IDE0,
            ide0: None,
            ide1: None,
            task_spawner: task_spawner.clone(),
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
            self.task_spawner.clone(),
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

    /// Advance the attached drives by one system step.
    pub fn tick(&mut self) {
        for ide in [&mut self.ide0, &mut self.ide1].iter_mut() {
            if let Some(ide) = ide.as_mut() {
                ide.tick();
            }
        }
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

                let val = ide.data_read8()?.clone();
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
        let ide_reg = ide.reg.read().unwrap().clone();

        match reg {
            Data => ide.data_read8(),
            Error | Features => Ok(ide_reg.error),
            SectorCount => Ok(ide_reg.sector_count),
            SectorNo | Lba0 => Ok(ide_reg.lba0_sector_no),
            CylinderLo | Lba1 => Ok(ide_reg.lba1_cyl_lo),
            CylinderHi | Lba2 => Ok(ide_reg.lba2_cyl_hi),
            DeviceHead | Lba3 => Ok(ide_reg.lba3_dev_head),
            Status | Command => {
                ide.irq.clear(); // ack IRQ
                Ok(ide_reg.status)
            }
            AltStatus | DevControl => Ok(ide_reg.status),
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
                return Ok(ide.reg.write().unwrap().lba3_dev_head = val);
            }
            _ => selected_ide!(self)?,
        };

        match reg {
            Data => ide.data_write8(val),
            Command | Status => ide.exec_cmd(val),

            _ => {
                let mut ide_reg = ide.reg.write().unwrap();
                match reg {
                    Features | Error => Ok(ide_reg.feature = val),
                    SectorCount => Ok(ide_reg.sector_count = val),
                    SectorNo | Lba0 => Ok(ide_reg.lba0_sector_no = val),
                    CylinderLo | Lba1 => Ok(ide_reg.lba1_cyl_lo = val),
                    CylinderHi | Lba2 => Ok(ide_reg.lba2_cyl_hi = val),
                    DeviceHead | Lba3 => unreachable!("should be handled above"),
                    DevControl | AltStatus => {
                        ide_reg.srst = val.get_bit(2);
                        ide_reg.nein = val.get_bit(1);
                        Ok(())
                    }
                    //DataLatch => Err(Unimplemented),
                    _ => Err(Unimplemented),
                }
            }
        }
    }
}

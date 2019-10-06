#![allow(clippy::cast_lossless)]
#![allow(dead_code)] // TODO: remove once project matures

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

use log::*;

mod debugger;
mod devices;
mod firmware;
mod memory;
mod ram;
mod registers;
mod util;

use arm7tdmi_rs::{reg, Cpu, Memory as ArmMemory, ARM_INIT};

use crate::devices::{FakeFlash, Flash, SysControl};
use crate::firmware::FirmwareInfo;
use crate::memory::{AccessViolation, AccessViolationKind, MemResultExt, Memory};
use crate::ram::Ram;
use crate::registers::CpuId;
use util::MemLogger;

use crate::debugger::asm2line::Asm2Line;

struct PP5020System {
    cpu: Cpu,
    cop: Cpu,
    devices: PP5020Devices,
}

#[derive(Debug)]
enum FatalError {
    CpuHalted,
    FatalAccessViolation(AccessViolation),
}

impl PP5020System {
    /// Returns a new PP5020System. Boots from a "cold start," running the
    /// bootloader code from the Flash ROM. Requires a copy of flash_rom dumped
    /// from iPod hardware.
    ///
    /// See https://www.rockbox.org/wiki/IpodFlash for details on how to obtain flash ROM dumps
    fn new(fw_file: impl Read, mut flash_rom: impl Read) -> std::io::Result<PP5020System> {
        // TODO: use the fw_file once HDD emulation is working
        let _ = fw_file;

        let mut data = Vec::new();
        flash_rom.read_to_end(&mut data)?;

        if data.len() != 0x10_0000 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "flash rom image must be 0x10000 bytes",
            ));
        }

        let devices = PP5020Devices::new(&data);
        let cpu = Cpu::new(ARM_INIT);
        let cop = Cpu::new(ARM_INIT);

        Ok(PP5020System { cpu, cop, devices })
    }

    /// Returns a new PP5020System using High Level Emulation (HLE) of the
    /// bootloader. Execution begins from OS code (as specified in the fw_file's
    /// structure), and the contents of Flash ROM are faked.
    fn new_hle(mut fw_file: impl Read + Seek) -> std::io::Result<PP5020System> {
        let fw_info = FirmwareInfo::new_from_reader(&mut fw_file)?;

        let os_image = fw_info.images().find(|img| img.name == *b"osos").unwrap();
        println!("Found OS Image: {:#x?}", os_image);

        // extract image from firmware
        fw_file.seek(SeekFrom::Start(os_image.dev_offset as u64 + 0x200))?;
        let mut os_image_data = vec![0; os_image.len as usize];
        fw_file.read_exact(&mut os_image_data)?;

        let mut devices = PP5020Devices::new_hle();
        devices.sdram.bulk_write(0, &os_image_data);

        // fake the bootloader, load directly at the image address
        let cpu = Cpu::new(&[
            (0, reg::PC, os_image.addr + os_image.entry_offset),
            (0, reg::CPSR, 0xd3),
        ]);
        let cop = Cpu::new(&[
            (0, reg::PC, os_image.addr + os_image.entry_offset),
            (0, reg::CPSR, 0xd3),
        ]);

        Ok(PP5020System { cpu, cop, devices })
    }

    /// Perform a HLE boot, but use an actual flash ROM image instead of a HLE
    /// Flash ROM.
    fn new_hle_with_flash(
        fw_file: impl Read + Seek,
        mut flash_rom: impl Read,
    ) -> std::io::Result<PP5020System> {
        let mut sys = PP5020System::new_hle(fw_file)?;

        let mut data = Vec::new();
        flash_rom.read_to_end(&mut data)?;

        if data.len() != 0x10_0000 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "flash rom image must be exactly 0x10000 bytes",
            ));
        }

        sys.devices.flash = Box::new(MemLogger::new(Flash::new(&data)));

        Ok(sys)
    }

    fn cycle(&mut self) -> Result<(), FatalError> {
        if self.devices.syscontrol.should_cycle_cpu() {
            // run the cpu
            if log::log_enabled!(log::Level::Trace) {
                eprint!("CPU: ");
            }
            self.cpu.cycle(self.devices.with_cpuid(CpuId::Cpu));
        }

        if let Some(access_violation) = self.devices.access_violation.take() {
            match access_violation.kind() {
                AccessViolationKind::Unimplemented => {
                    return Err(FatalError::FatalAccessViolation(access_violation))
                }
                AccessViolationKind::Misaligned => {
                    log::warn!("CPU {:#010x?}", access_violation);
                    // FIXME: Misaligned access (i.e: Data Abort) _should_ be recoverable.
                    // ...but for early development, it's probably more likely than not that a
                    // misaligned access is a side-effect of my bad code, _not_ application code
                    // running inside the emulator doing Bad Things.
                    return Err(FatalError::FatalAccessViolation(access_violation));
                }
            }
        }

        if self.devices.syscontrol.should_cycle_cop() {
            // run the cop
            if log::log_enabled!(log::Level::Trace) {
                eprint!("COP: ");
            }
            self.cop.cycle(self.devices.with_cpuid(CpuId::Cop));
        }

        Ok(())
    }
}

struct PP5020Devices {
    pub access_violation: Option<AccessViolation>,
    stub: devices::Stub,

    pub flash: Box<dyn Memory>, // 1 MB
    pub sdram: Ram,             // 32 MB
    pub fastram: Ram,           // 96 KB
    pub syscontrol: SysControl,
}

impl PP5020Devices {
    fn new(flash_rom: &[u8]) -> PP5020Devices {
        PP5020Devices {
            access_violation: None,
            stub: devices::Stub,

            flash: Box::new(Flash::new(flash_rom)),
            sdram: Ram::new(32 * 1024 * 1024),
            fastram: Ram::new(96 * 1024),
            syscontrol: SysControl::new(),
        }
    }

    fn new_hle() -> PP5020Devices {
        PP5020Devices {
            access_violation: None,
            stub: devices::Stub,

            flash: Box::new(FakeFlash::new()),
            sdram: Ram::new(32 * 1024 * 1024),
            fastram: Ram::new(96 * 1024),
            syscontrol: SysControl::new(),
        }
    }

    fn with_cpuid(&mut self, cpuid: CpuId) -> &mut Self {
        self.syscontrol.set_cpuid(cpuid);
        self
    }

    // TODO: explore other ways of specifying memory map, preferably _without_
    // trait objects (or at the very least, without having to constantly remake
    // the exact same trait object on each call)

    fn addr_to_mem_offset(&mut self, addr: u32) -> (&mut dyn Memory, u32) {
        match addr {
            0x0000_0000..=0x000f_ffff => (&mut self.flash, 0),
            // ???
            0x1000_0000..=0x3fff_ffff => (&mut self.sdram, 0x1000_0000),
            0x4000_0000..=0x4001_7fff => (&mut self.fastram, 0x4000_0000),
            // ???
            0x6000_0000..=0x6fff_ffff => (&mut self.syscontrol, 0x6000_0000),
            // ???
            _ => (&mut self.stub, 0),
        }
    }
}

// Because the arm7tdmi cpu expects all memory accesses to succeed, there needs
// to be a shim between Clicky's memory interface (which is fallible), and the
// arm7tdmi-rs Memory interface.

macro_rules! impl_arm7tdmi_r {
    ($fn:ident, $ret:ty) => {
        fn $fn(&mut self, addr: u32) -> $ret {
            let (mem, offset) = self.addr_to_mem_offset(addr);
            mem.$fn(addr - offset)
                .map_memerr_offset(offset)
                .map_err(|e| self.access_violation = Some(e))
                .unwrap_or(0x00) // contents of register undefined
        }
    };
}

macro_rules! impl_arm7tdmi_w {
    ($fn:ident, $val:ty) => {
        fn $fn(&mut self, addr: u32, val: $val) {
            let (mem, offset) = self.addr_to_mem_offset(addr);
            mem.$fn(addr - offset, val as $val)
                .map_memerr_offset(offset)
                .map_err(|e| self.access_violation = Some(e))
                .unwrap_or(())
        }
    };
}

impl ArmMemory for PP5020Devices {
    impl_arm7tdmi_r!(r8, u8);
    impl_arm7tdmi_r!(r16, u16);
    impl_arm7tdmi_r!(r32, u32);
    impl_arm7tdmi_w!(w8, u8);
    impl_arm7tdmi_w!(w16, u16);
    impl_arm7tdmi_w!(w32, u32);
}

fn main() -> std::io::Result<()> {
    pretty_env_logger::init();

    let args: Vec<String> = std::env::args().collect();

    let fw_file = File::open(&args[1])?;
    let mut system = match args.get(2).map(|s| s.as_str()) {
        None | Some("hle") => PP5020System::new_hle(fw_file)?,
        Some(s) => PP5020System::new_hle_with_flash(fw_file, File::open(s)?)?,
    };

    let mut debugger = None;
    if let Some(objdump_fname) = args.get(3) {
        let mut dbg = Asm2Line::new();
        dbg.load_objdump(objdump_fname)?;
        debugger = Some(dbg)
    }

    let mut step_through = true;
    loop {
        let pc = system.cpu.reg_get(0, reg::PC);

        if let Err(fatal_error) = system.cycle() {
            eprintln!("Fatal Error Occurred!");
            eprintln!("{:#x?}", system.cpu);
            eprintln!("{:#010x?}", fatal_error);
            if let Some(ref mut debugger) = debugger {
                match debugger.lookup(pc) {
                    Some(info) => eprintln!("{}", info),
                    None => eprintln!("???"),
                }
            }
            panic!();
        }

        #[allow(clippy::single_match)]
        match pc {
            0x1000_0474 => step_through = true, // right after relocate to 0x40000000
            0x4000_0098 => step_through = true, // right after init_bss
            _ => {}
        }

        // quick-and-dirty step through
        if step_through {
            if let Some(ref mut debugger) = debugger {
                match debugger.lookup(pc) {
                    Some(info) => debug!("{}", info),
                    None => debug!("???"),
                }
            }

            let c = std::io::stdin().bytes().next().unwrap().unwrap();
            match c as char {
                'r' => step_through = false,
                's' => step_through = true,
                _ => {}
            }
        }
    }
}

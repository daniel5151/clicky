#![allow(clippy::cast_lossless)]
#![allow(dead_code)] // TODO: remove once project matures

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

mod debugger;
mod devices;
mod firmware;
mod memory;
mod ram;
mod registers;

use arm7tdmi_rs::{reg, Cpu, Memory};

use crate::devices::FakeFlash;
use crate::firmware::FirmwareInfo;
use crate::memory::WordAligned;
use crate::ram::Ram;
use crate::registers::{CpuController, CpuId};

use crate::debugger::asm2line::Asm2Line;

struct PP5020System {
    cpu: Cpu,
    cop: Cpu,
    devices: PP5020Devices,
}

impl PP5020System {
    fn new_hle_boot(mut fw_file: impl Read + Seek) -> std::io::Result<PP5020System> {
        let fw_info = FirmwareInfo::new_from_reader(&mut fw_file)?;

        let os_image = fw_info.images().find(|img| img.name == *b"osos").unwrap();
        println!("Found OS Image: {:#x?}", os_image);

        // extract image from firmware
        fw_file.seek(SeekFrom::Start(os_image.dev_offset as u64 + 0x200))?;
        let mut os_image_data = vec![0; os_image.len as usize];
        fw_file.read_exact(&mut os_image_data)?;

        let mut devices = PP5020Devices::new();
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

    fn cycle(&mut self) {
        // Check / Update CPU Controllers
        // TODO: implement the controllers properly
        use crate::registers::cpu_controller::flags as cpuctl;
        if self.devices.cpu_controller.raw() & cpuctl::FLOW_MASK == 0 {
            // run the cpu
            if log::log_enabled!(log::Level::Trace) {
                eprint!("CPU: ");
            }
            self.cpu.cycle(self.devices.with_cpuid(CpuId::Cpu));
        }

        if self.devices.cop_controller.raw() & cpuctl::FLOW_MASK == 0 {
            // run the cop
            if log::log_enabled!(log::Level::Trace) {
                eprint!("COP: ");
            }
            self.cop.cycle(self.devices.with_cpuid(CpuId::Cop));
        }
    }
}

struct PP5020Devices {
    pub bad_mem_access: Option<u32>,

    pub fakeflash: WordAligned<FakeFlash>,
    pub sdram: Ram,   // 32 MB
    pub fastram: Ram, // 96 KB
    pub cpuid: WordAligned<CpuId>,
    pub cpu_controller: WordAligned<CpuController>,
    pub cop_controller: WordAligned<CpuController>,
}

impl PP5020Devices {
    fn new() -> PP5020Devices {
        PP5020Devices {
            bad_mem_access: None,

            fakeflash: WordAligned::new(FakeFlash::new()),
            sdram: Ram::new(32 * 1024 * 1024),
            fastram: Ram::new(96 * 1024),
            cpuid: WordAligned::new(CpuId::Cpu),
            cpu_controller: WordAligned::new(CpuController::new()),
            cop_controller: WordAligned::new(CpuController::new()),
        }
    }

    fn with_cpuid(&mut self, cpuid: CpuId) -> &mut Self {
        *self.cpuid = cpuid;
        self
    }

    // TODO: explore other ways of specifying memory map, preferably _without_
    // trait objects (or, at the very least, wihtout having to constantly remake the
    // same trait object)

    fn addr_to_mem(&mut self, addr: u32) -> (&mut dyn Memory, u32) {
        match addr {
            0x0000_0000..=0x0fff_ffff => (&mut self.fakeflash, addr),
            0x1000_0000..=0x3fff_ffff => (&mut self.sdram, addr - 0x1000_0000),
            0x4000_0000..=0x4001_7fff => (&mut self.fastram, addr - 0x4000_0000),
            0x6000_0000 => (&mut self.cpuid, addr - 0x6000_0000),
            0x6000_7000 => (&mut self.cpu_controller, addr - 0x6000_7000),
            0x6000_7004 => (&mut self.cop_controller, addr - 0x6000_7004),
            _ => {
                self.bad_mem_access = Some(addr);
                // just return _something_. not like it matters, since we are
                // gonna exit after this instruction anyways
                (&mut self.cpuid, 0)
            }
        }
    }
}

impl Memory for PP5020Devices {
    fn r8(&mut self, addr: u32) -> u8 {
        let (mem, addr) = self.addr_to_mem(addr);
        mem.r8(addr)
    }

    fn r16(&mut self, addr: u32) -> u16 {
        let (mem, addr) = self.addr_to_mem(addr);
        mem.r16(addr)
    }

    fn r32(&mut self, addr: u32) -> u32 {
        let (mem, addr) = self.addr_to_mem(addr);
        mem.r32(addr)
    }

    fn w8(&mut self, addr: u32, val: u8) {
        let (mem, addr) = self.addr_to_mem(addr);
        mem.w8(addr, val)
    }

    fn w16(&mut self, addr: u32, val: u16) {
        let (mem, addr) = self.addr_to_mem(addr);
        mem.w16(addr, val)
    }

    fn w32(&mut self, addr: u32, val: u32) {
        let (mem, addr) = self.addr_to_mem(addr);
        mem.w32(addr, val)
    }
}

fn main() -> std::io::Result<()> {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();

    let fw_file = File::open(&args[1])?;

    let mut debugger = None;
    if let Some(objdump_fname) = args.get(2) {
        let mut dbg = Asm2Line::new();
        dbg.load_objdump(objdump_fname)?;
        debugger = Some(dbg)
    }

    let mut system = PP5020System::new_hle_boot(fw_file)?;

    let mut step_through = false;
    loop {
        let pc = system.cpu.reg_get(0, reg::PC);

        system.cycle();

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
                    Some(info) => println!("{}", info),
                    None => println!("???"),
                }
            }

            let c = std::io::stdin().bytes().next().unwrap().unwrap();
            match c as char {
                'r' => step_through = false,
                's' => step_through = true,
                _ => {}
            }
        }

        if let Some(addr) = system.devices.bad_mem_access {
            eprintln!("accessed unimplemented addr {:#010x}", addr);
            eprintln!("{:#x?}", system.cpu);
            if let Some(ref mut debugger) = debugger {
                match debugger.lookup(pc) {
                    Some(info) => eprintln!("{}", info),
                    None => eprintln!("???"),
                }
            }
            panic!();
        }
    }
}

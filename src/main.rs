#![allow(clippy::cast_lossless)]
#![allow(dead_code)] // TODO: remove once project matures

use std::fs::File;

mod devices;
mod firmware;
mod ram;

use arm7tdmi_rs::{reg, Cpu, Memory};

use crate::devices::cpuid::CpuId;
use crate::devices::Device;
use crate::firmware::FirmwareInfo;
use crate::ram::Ram;

struct PP5020 {
    sdram: Ram,   // 32 MB
    fastram: Ram, // 96 KB
    cpuid: Device<CpuId>,
}

impl PP5020 {
    fn new() -> PP5020 {
        PP5020 {
            sdram: Ram::new(32 * 1024 * 1024),
            fastram: Ram::new(96 * 1024),
            cpuid: Device::new(CpuId::Cpu),
        }
    }

    fn set_cpuid(&mut self, cpuid: CpuId) -> &mut Self {
        *self.cpuid.as_mut() = cpuid;
        self
    }

    // TODO: explore other ways of specifying memory map, preferably _without_
    // trait objects

    fn addr_to_mem(&mut self, addr: u32) -> (&mut dyn Memory, u32) {
        match addr {
            0x0000_0000..=0x0fff_ffff => panic!("accessed Flash address {:#08x}", addr),
            0x1000_0000..=0x3fff_ffff => (&mut self.sdram, addr - 0x1000_0000),
            0x4000_0000..=0x4001_7fff => (&mut self.fastram, addr - 0x4000_0000),
            0x6000_0000 => (&mut self.cpuid, addr - 0x6000_0000),
            _ => panic!("accessed unimplemented addr {:#08x}", addr),
        }
    }
}

impl Memory for PP5020 {
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

use std::io::{Read, Seek, SeekFrom};

fn main() -> std::io::Result<()> {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();

    let mut fw_file = File::open(&args[1])?;
    let fw_info = FirmwareInfo::new_from_reader(&mut fw_file)?;

    let os_image = fw_info.images().find(|img| img.name == *b"osos").unwrap();

    println!("Found OS Image: {:#x?}", os_image);

    // extract image from firmware
    fw_file.seek(SeekFrom::Start(os_image.dev_offset as u64 + 0x200))?;
    let mut os_image_data = vec![0; os_image.len as usize];
    fw_file.read_exact(&mut os_image_data)?;

    // fake the bootloader, load directly at the image address
    let mut mem = PP5020::new();
    mem.sdram.bulk_write(0, &os_image_data);

    let mut cpu = Cpu::new(&[
        (0, reg::PC, os_image.addr + os_image.entry_offset),
        (0, reg::CPSR, 0xd3),
    ]);
    let mut cop = Cpu::new(&[
        (0, reg::PC, os_image.addr + os_image.entry_offset),
        (0, reg::CPSR, 0xd3),
    ]);

    loop {
        // TODO: find a cleaner way to giving each CPU a _slightly_ different mmu
        eprint!("CPU: ");
        cpu.cycle(mem.set_cpuid(CpuId::Cpu));
        eprint!("COP: ");
        cop.cycle(mem.set_cpuid(CpuId::Cop));

        // quick-and-dirty step through
        let _ = std::io::stdin().read(&mut [0u8]).unwrap();
        // disasm last instruction
    }
}

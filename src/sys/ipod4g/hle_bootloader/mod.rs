use std::io::{Read, Seek, SeekFrom};

use armv4t_emu::{reg, Mode as ArmMode};

use crate::memory::Memory;

use super::Ipod4g;

mod firmware;
mod sysinfo;

use sysinfo::sysinfo_t;

/// Put the system into a state as though the bootloader in Flash ROM was run.
pub(super) fn run_hle_bootloader(
    ipod: &mut Ipod4g,
    mut fw_file: impl Read + Seek,
) -> Result<(), Box<dyn std::error::Error>> {
    if ipod.devices.flash.is_hle() {
        warn!("Running HLE bootloader even though the system is using a real Flash ROM dump!");
    }

    let fw_info = firmware::FirmwareMeta::parse(&mut fw_file)?;

    info!("Parsed firmware meta: {:#x?}", fw_info);

    let os_image = fw_info
        .images
        .iter()
        .find(|img| img.name == *b"osos")
        .ok_or("could not find OS image")?;

    // extract image from firmware file, and copy it into RAM
    fw_file.seek(SeekFrom::Start(os_image.dev_offset as u64 + 0x200))?;
    let mut os_image_data = vec![0; os_image.len as usize];
    fw_file.read_exact(&mut os_image_data)?;

    ipod.devices.sdram.bulk_write(0, &os_image_data);

    // set the CPU to start execution from the image entry address
    ipod.cpu.reg_set(
        ArmMode::User,
        reg::PC,
        os_image.addr + os_image.entry_offset,
    );
    ipod.cpu.reg_set(ArmMode::User, reg::CPSR, 0xd3); // supervisor mode
    ipod.cop = ipod.cpu;

    // inject some HLE CPU state
    ipod.cpu.reg_set(ArmMode::Irq, reg::SP, 0x40017bfc);

    // inject fake sysinfo into fastram.
    //
    // I threw my copy of the iPod 4g flashROM into Ghidra, and as far as I can
    // tell, the bootloader does indeed set this structure up somewhere in memory.
    // I don't _fully_ understand where ipodloader got this magic pointer address
    // from, because perusing the flash ROM disassembly didn't reveal any
    // immediately obvious writes to that address.
    //
    // Anyhoo, I gave up on doing it "correctly" and kinda just futzed around with
    // the addresses until the code managed to progress further. I _hope_ this
    // structure isn't used past the init stage, since I picked the memory location
    // to write it into somewhat arbitrarily, and there's no reason some other code
    // might not come in and trash it...
    //
    // TODO?: add some sort of signaling system if the sysinfo struct is overwritten
    const SYSINFO_PTR: u32 = 0x4001_7f1c;
    // SYSINFO_LOC is pulled out of my ass lol
    const SYSINFO_LOC: u32 = 0x4001_7f00 - std::mem::size_of::<sysinfo_t>() as u32;
    ipod.devices.w32(SYSINFO_PTR, SYSINFO_LOC).unwrap(); // pointer to sysinfo
    ipod.devices.fastram.bulk_write(
        SYSINFO_LOC - 0x4000_0000,
        // FIXME?: this will break on big-endian systems
        bytemuck::bytes_of(&sysinfo_t {
            IsyS: u32::from_le_bytes(*b"IsyS"),
            len: 0x184,
            boardHwSwInterfaceRev: 0x50000,
            ..Default::default()
        }),
    );

    Ok(())
}

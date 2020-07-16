use std::io::{Read, Seek, SeekFrom};

use armv4t_emu::{reg, Mode as ArmMode};
use std::io;
use thiserror::Error;

use crate::memory::Memory;

use super::Ipod4g;

mod firmware;
mod sysinfo;

use sysinfo::sysinfo_t;

#[derive(Error, Debug)]
pub enum HleBootloaderError {
    #[error("error while reading firmware file")]
    Io(#[from] io::Error),
    #[error("magic_hi != \"[hi]\" in the volume header")]
    BadMagic,
    #[error("HLE boot for firmware version {0} isn't (currently) supported")]
    InvalidVersion(u16),
    #[error("Couldn't find valid `osos` image")]
    MissingOs,
}

/// Put the system into a state as though the bootloader in Flash ROM was run.
pub(super) fn run_hle_bootloader(
    ipod: &mut Ipod4g,
    mut fw_file: impl Read + Seek,
) -> Result<(), HleBootloaderError> {
    if ipod.devices.flash.is_hle() {
        warn!("Running HLE bootloader even though the system is using a real Flash ROM dump!");
    }

    let fw_info = firmware::FirmwareMeta::parse(&mut fw_file)?;

    info!("Parsed firmware meta: {:#x?}", fw_info);

    let os_image = fw_info
        .images
        .iter()
        .find(|img| img.name == *b"osos")
        .ok_or(HleBootloaderError::MissingOs)?;

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

    // inject fake sysinfo_t into fastram.
    // TODO: look into how this pointer changes between iPod models
    const SYSINFO_PTR: u32 = 0x4001_7f1c;
    const SYSINFO_LOC: u32 = 0x4000_ff18;
    ipod.devices.w32(SYSINFO_PTR, SYSINFO_LOC).unwrap(); // pointer to sysinfo
    ipod.devices.fastram.bulk_write(
        SYSINFO_LOC - 0x4000_0000,
        // FIXME?: this will break on big-endian systems
        bytemuck::bytes_of(&sysinfo_t {
            IsyS: u32::from_le_bytes(*b"IsyS"),
            len: 0x184,
            boardHwSwInterfaceRev: 0x50014,
            ..Default::default()
        }),
    );

    // The bootloader enables the GPIOA:5 pin (i.e: the Hold button)
    ipod.devices
        .gpio_abcd
        .lock()
        .unwrap()
        .w32(0x00, 0x20)
        .unwrap();

    Ok(())
}

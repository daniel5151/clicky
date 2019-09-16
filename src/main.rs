#![allow(clippy::cast_lossless)]
#![allow(dead_code)] // TODO: remove once project matures

use std::fs::File;

mod firmware;
mod ram;

use crate::firmware::FirmwareInfo;
fn main() -> std::io::Result<()> {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    let fw_info = FirmwareInfo::new_from_reader(&mut File::open(&args[1])?)?;

    let os_image = fw_info.images().find(|img| img.name == *b"osos").unwrap();

    println!("Found OS Image: {:#x?}", os_image);

    Ok(())
}

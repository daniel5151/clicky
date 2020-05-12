use std::io::{self, Read, Seek, SeekFrom};

use byteorder::{BigEndian, ByteOrder, ReadBytesExt, LE};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("error while reading firmware file")]
    Io(#[from] io::Error),
    #[error("magic_hi != \"[hi]\" in the volume header")]
    BadMagic,
    #[error("Firmware version {0} isn't (currently) supported")]
    InvalidVersion(u16),
}

/// Firmware image metadata.
///
/// See http://www.ipodlinux.org/Firmware.html
#[derive(Debug)]
pub struct FirmwareMeta {
    pub header: VolumeHeader,
    pub images: Vec<ImageInfo>,
}

impl FirmwareMeta {
    pub fn parse(fw: &mut (impl Read + Seek)) -> Result<FirmwareMeta, Error> {
        // Volume Header
        let header = VolumeHeader::parse(fw)?;
        if header.magic_hi != BigEndian::read_u32(b"[hi]") {
            return Err(Error::BadMagic);
        }

        // TODO: don't assume FW version is 3, as each fw uses slightly-different
        // offsets between things
        if header.format_version != 3 {
            return Err(Error::InvalidVersion(header.format_version));
        }

        // Pull directory entries
        fw.seek(SeekFrom::Start(header.dir_offset as u64 + 0x200))?;

        let mut images = Vec::new();
        loop {
            let image = ImageInfo::parse(fw)?;
            if image.dev == *b"\0\0\0\0" {
                break;
            }
            images.push(image)
        }

        Ok(FirmwareMeta { header, images })
    }
}

#[derive(Debug)]
pub struct VolumeHeader {
    pub magic_hi: u32,
    pub dir_offset: u32,
    pub ext_header_loc: u16,
    pub format_version: u16,
}

impl VolumeHeader {
    fn parse(rdr: &mut impl Read) -> io::Result<VolumeHeader> {
        // this is just a static string, which we can skip over. we _should_ make sure
        // that it matches the expected STOP string, but that's overkill...
        let mut stop = vec![0; 256];
        rdr.read_exact(&mut stop)?;

        #[rustfmt::skip]
        let header = VolumeHeader {
            magic_hi:       rdr.read_u32::<LE>()?,
            dir_offset:     rdr.read_u32::<LE>()?,
            ext_header_loc: rdr.read_u16::<LE>()?,
            format_version: rdr.read_u16::<LE>()?,
        };

        Ok(header)
    }
}

#[derive(Debug)]
pub struct ImageInfo {
    pub dev: [u8; 4],
    pub name: [u8; 4],
    pub id: u32,
    pub dev_offset: u32,
    pub len: u32,
    pub addr: u32,
    pub entry_offset: u32,
    pub checksum: u32,
    pub vers: u32,
    pub load_addr: u32,
}

impl ImageInfo {
    fn parse(rdr: &mut impl Read) -> io::Result<ImageInfo> {
        #[rustfmt::skip]
        let image = ImageInfo {
            dev:          rdr.read_u32::<LE>()?.to_be_bytes(),
            name:         rdr.read_u32::<LE>()?.to_be_bytes(),
            id:           rdr.read_u32::<LE>()?,
            dev_offset:   rdr.read_u32::<LE>()?,
            len:          rdr.read_u32::<LE>()?,
            addr:         rdr.read_u32::<LE>()?,
            entry_offset: rdr.read_u32::<LE>()?,
            checksum:     rdr.read_u32::<LE>()?,
            vers:         rdr.read_u32::<LE>()?,
            load_addr:    rdr.read_u32::<LE>()?,
        };

        Ok(image)
    }
}

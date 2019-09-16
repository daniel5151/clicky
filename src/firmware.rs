use std::io;
use std::io::{Read, Seek, SeekFrom};

use byteorder::{BigEndian, ByteOrder, ReadBytesExt, LE};
use derivative::Derivative;

/// Information extracted from an iPod firmware image
#[derive(Debug)]
pub struct FirmwareInfo {
    version: u16,
    ext_header_loc: u16,
    images: Vec<ImageInfo>,
}

impl FirmwareInfo {
    pub fn new_from_reader(fw: &mut (impl Read + Seek)) -> io::Result<FirmwareInfo> {
        // Volume Header
        let header = VolumeHeader::from_reader(fw)?;
        if header.magic_hi != BigEndian::read_u32(b"[hi]") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "magic_hi != \"[hi]\" in the volume header",
            ));
        }

        // TODO: don't assume FW version is 3, as each fw uses slightly-different
        // offsets between things

        // Pull directory entries
        fw.seek(SeekFrom::Start(header.dir_offset as u64 + 0x200))?;

        let mut images = Vec::new();
        loop {
            let image = ImageInfo::from_reader(fw)?;
            if image.dev == *b"\0\0\0\0" {
                break;
            }
            images.push(image)
        }

        Ok(FirmwareInfo {
            version: header.format_version,
            ext_header_loc: header.ext_header_loc,
            images,
        })
    }

    pub fn version(&self) -> u16 {
        self.version
    }

    pub fn ext_header_loc(&self) -> u16 {
        self.ext_header_loc
    }

    pub fn images(&self) -> impl Iterator<Item = &ImageInfo> {
        self.images.iter()
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
struct VolumeHeader {
    #[derivative(Debug(format_with = "fmt_stop_header"))]
    stop: Vec<u8>, // 256 bytes
    magic_hi: u32,
    dir_offset: u32,
    ext_header_loc: u16,
    format_version: u16,
}

impl VolumeHeader {
    pub fn from_reader(rdr: &mut impl Read) -> io::Result<VolumeHeader> {
        let mut stop = vec![0; 256];
        rdr.read_exact(&mut stop)?;

        #[rustfmt::skip]
        let header = VolumeHeader {
            stop,
            magic_hi:       rdr.read_u32::<LE>()?,
            dir_offset:     rdr.read_u32::<LE>()?,
            ext_header_loc: rdr.read_u16::<LE>()?,
            format_version: rdr.read_u16::<LE>()?,
        };

        Ok(header)
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct ImageInfo {
    #[derivative(Debug(format_with = "fmt_u8_slice_as_ascii"))]
    pub dev: [u8; 4],
    #[derivative(Debug(format_with = "fmt_u8_slice_as_ascii"))]
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
    fn from_reader(rdr: &mut impl Read) -> io::Result<ImageInfo> {
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

fn fmt_stop_header(stop: &[u8], f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
    if f.alternate() {
        f.write_str("\"")?;
        for chunk in stop.chunks_exact(16) {
            writeln!(f)?;
            for &b in chunk {
                write!(f, "{}", b as char)?;
            }
        }
        f.write_str("\"")?;
    } else {
        write!(f, "{{...}}")?
    }
    Ok(())
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn fmt_u8_slice_as_ascii(v: &[u8; 4], f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
    write!(f, "b\"{}\"", String::from_utf8_lossy(v))
}

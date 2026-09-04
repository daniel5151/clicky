//! SCfg/SysCfg table, key/value store for serial number, model number, etc.
//!
//! 24-byte header sits at `0x2000`, followed by fixed 20-byte records:
//!
//! ```text
//! header
//! +0  tag "SCfg"
//! +4  u32 total length
//! +8  u32 base address
//! +12 u16,u16 version (?)
//! +16 u32 (zero)
//! +20 u32 record count
//! ```
//!
//! ```text
//! record
//! +0  tag
//! +4  value (16B)
//! ```
//!
//! Tags are stored in reverse endianness (SCfg -> gfCS)

use std::convert::TryInto;

/// How far into the dump to look for the header. The table lives early, and
/// bounding the scan avoids matching the copies in mirrored banks further in.
const SCAN_LIMIT: usize = 0x1_0000;

const HEADER_LEN: usize = 24;
const RECORD_LEN: usize = 20;
const VALUE_LEN: usize = RECORD_LEN - 4;
const SCFG_TAG: &[u8; 4] = b"SCfg";

fn u32le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes.try_into().unwrap())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub tag: [u8; 4],
    pub value: [u8; VALUE_LEN],
}

impl Record {
    pub fn tag_str(&self) -> &str {
        std::str::from_utf8(&self.tag).unwrap_or("????")
    }

    /// Word `n` of the value, little-endian. Panics if `n > 3`.
    pub fn word(&self, n: usize) -> u32 {
        u32le(&self.value[n * 4..n * 4 + 4])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SysCfg {
    pub version: (u16, u16),
    pub records: Vec<Record>,
}

impl SysCfg {
    pub fn from_rom(rom: &[u8]) -> Option<SysCfg> {
        let needle = [SCFG_TAG[3], SCFG_TAG[2], SCFG_TAG[1], SCFG_TAG[0]];
        let limit = rom.len().min(SCAN_LIMIT);
        let offset = rom[..limit].windows(4).position(|w| w == needle)?;
        let header = rom.get(offset..offset + HEADER_LEN)?;

        let len = u32le(&header[4..8]) as usize;
        //let base = u32le(&header[8..12]);
        let version = (
            u16::from_le_bytes(header[12..14].try_into().ok()?),
            u16::from_le_bytes(header[14..16].try_into().ok()?),
        );
        let count = u32le(&header[20..24]) as usize;

        // Check that length reported in the header matches the computed size
        if len != HEADER_LEN + count * RECORD_LEN {
            return None;
        }

        let mut records = Vec::with_capacity(count);
        for i in 0..count {
            let at = offset + HEADER_LEN + i * RECORD_LEN;
            let raw = rom.get(at..at + RECORD_LEN)?;
            records.push(Record {
                tag: [raw[3], raw[2], raw[1], raw[0]],
                value: raw[4..].try_into().ok()?,
            });
        }

        Some(SysCfg {
            version,
            records,
        })
    }

    /// Look up a record by tag (e.g. `b"HwVr"`)
    pub fn get(&self, tag: &[u8; 4]) -> Option<&Record> {
        self.records.iter().find(|r| &r.tag == tag)
    }
}

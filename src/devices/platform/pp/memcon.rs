use log::Level::*;

use crate::devices::{Device, Probe};
use crate::memory::{MemException::*, MemResult, Memory};

/// Memory Protection Bits
#[derive(Debug)]
pub struct Protection {
    pub r: bool,
    pub w: bool,
    /// what the heck is a data mask?
    pub d: bool,
    pub x: bool,
}

#[derive(Debug, Default)]
struct Mmap {
    // XXX: these shouldn't be options. I just don't know what the HLE vals at init are, and this
    // is a hacky side-step for that issue.
    logical: u32,
    physical: u32,
}

/// PP5020 Memory Controller. Content varies based on which CPU/COP is
/// performing the access.
///
/// Shoutout to the mysterious MrH for reverse-engineering most of this info!
/// https://daniel.haxx.se/sansa/memory_controller.txt
pub struct MemCon {
    cache_data: [u32; 0x2000],
    /// A status word is 32 bits and is mirrored four times for each cache line
    ///
    /// bit 0-20    line_address >> 11
    /// bit 21      unused?
    /// bit 22      line_dirty
    /// bit 23      line_valid
    /// bit 24-31   unused?
    cache_status: [u32; 0x2000],

    mmap: [Mmap; 8],
    cache_mask: u32,
    /// Set back to zero after use
    cache_flush_mask: u32,
}

impl std::fmt::Debug for MemCon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("MemCon")
            .field(
                "cache_data",
                &format!(
                    "[{:#010x?}, {:#010x?}, ...; 0x2000]",
                    self.cache_data[0], self.cache_data[1]
                ),
            )
            .field(
                "cache_status",
                &format!(
                    "[{:#010x?}, {:#010x?}, ...; 0x2000]",
                    self.cache_status[0], self.cache_status[1]
                ),
            )
            .field("mmap", &self.mmap)
            .field("cache_mask", &self.cache_mask)
            .field("cache_flush_mask", &self.cache_flush_mask)
            .finish()
    }
}

impl MemCon {
    pub fn new() -> MemCon {
        MemCon {
            cache_data: [0; 0x2000],
            cache_status: [0; 0x2000],
            mmap: Default::default(),
            cache_mask: 0,
            cache_flush_mask: 0,
        }
    }

    pub fn virt_to_phys(&self, addr: u32) -> (u32, Protection) {
        // TODO: debug this :(

        // use bit_field::BitField;
        // for Mmap { logical, physical } in self.mmap.iter() {
        //     // XXX: see note why these are options
        //     let (logical, physical) = match (logical, physical) {
        //         (Some(x), Some(y)) => (x, y),
        //         _ => continue,
        //     };

        //     let mask = logical.get_bits(0..=13) << 16;
        //     let virt_addr = logical.get_bits(16..=29) << 16;
        //     let phys_addr = physical.get_bits(16..=29) << 16;
        //     let prot = Protection {
        //         r: physical.get_bit(8),
        //         w: physical.get_bit(9),
        //         d: physical.get_bit(10),
        //         x: physical.get_bit(11),
        //     };

        //     debug!(
        //         "[{:#010x?}:virt, {:#010x?}:phys] ({:#010x?}:addr & {:#010x?}:mask =
        // {:x?}) ({:#010x?}:virt_addr & mask = {:x?})",         virt_addr,
        //         phys_addr,
        //         addr,
        //         mask,
        //         addr & mask,
        //         virt_addr,
        //         virt_addr & mask,
        //     );

        //     // If I'm reading the docs right, then this is the code that aught to
        // work.     // Unfortunately, it doesn't, and I'm not sure why...
        //     let final_addr = {
        //         if (addr & mask) != (virt_addr & mask) {
        //             continue;
        //         }
        //         (addr & !mask) | (phys_addr & mask)
        //     };

        //     debug!("{:x?} -> {:x?}", addr, final_addr);

        //     return (final_addr, prot);
        // }

        // no mapping, just use default options
        (
            addr,
            Protection {
                r: true,
                w: true,
                d: true,
                x: true,
            },
        )
    }
}

impl Device for MemCon {
    fn kind(&self) -> &'static str {
        "Memory Controller"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x0000..=0x1fff => "<cache data>",
            0x2000..=0x3fff => "<cache data mirror>",
            0x4000..=0x5fff => "<cache status>",
            0x6000..=0x7fff => "<cache status mirror>",
            0x8000..=0x9fff => "<cache flush?>",
            0xa000..=0xbfff => "<cache flush mirror?>",
            0xc000..=0xdfff => "<cache invalidate?>",
            0xe000..=0xefff => "?",
            0xf000..=0xf03f if offset & 0b100 == 0 => "Mmap<X>Logical",
            0xf000..=0xf03f if offset & 0b100 != 0 => "Mmap<X>Physical",
            0xf040 => "CacheMask",
            0xf044 => "CacheControl",
            0xf048 => "CacheFlushMask",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for MemCon {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x0000..=0x1fff => Err(Unimplemented),
            0x2000..=0x3fff => Err(Unimplemented),
            0x4000..=0x5fff => Err(Unimplemented),
            0x6000..=0x7fff => Err(Unimplemented),
            0x8000..=0x9fff => Err(InvalidAccess),
            0xa000..=0xbfff => Err(InvalidAccess),
            0xc000..=0xdfff => Err(InvalidAccess),
            0xf000..=0xf03f if offset & 0b100 == 0 => {
                let no = (offset - 0xf000) / 8;
                Ok(self.mmap[no as usize].logical)
            }
            0xf000..=0xf03f if offset & 0b100 != 0 => {
                let no = (offset - 0xf000) / 8;
                Ok(self.mmap[no as usize].physical)
            }
            0xf040 => Err(StubRead(Info, self.cache_mask)),
            0xf044 => Err(InvalidAccess),
            0xf048 => Err(Unimplemented),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x0000..=0x1fff => Err(Unimplemented),
            0x2000..=0x3fff => Err(Unimplemented),
            0x4000..=0x5fff => Err(Unimplemented),
            0x6000..=0x7fff => Err(Unimplemented),
            0x8000..=0x9fff => Err(StubWrite(Info, ())),
            0xa000..=0xbfff => Err(StubWrite(Info, ())),
            0xc000..=0xdfff => Err(StubWrite(Info, ())),
            0xf000..=0xf03f if offset & 0b100 == 0 => {
                let no = (offset - 0xf000) / 8;
                self.mmap[no as usize].logical = val;
                Err(FatalError("mmap not implemented".into()))
            }
            0xf000..=0xf03f if offset & 0b100 != 0 => {
                let no = (offset - 0xf000) / 8;
                self.mmap[no as usize].physical = val;
                Err(FatalError("mmap not implemented".into()))
            }
            0xf040 => Err(StubWrite(Info, self.cache_mask = val)),
            0xf044 => Err(StubWrite(Info, ())),
            0xf048 => Err(Unimplemented),
            _ => Err(Unexpected),
        }
    }
}

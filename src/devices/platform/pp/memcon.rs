use crate::devices::prelude::*;

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
    logical: u32,
    physical: u32,
}

/// PP5020 Memory Controller. Content varies based on which CPU/COP is
/// performing the access.
///
/// Shoutout to the mysterious MrH for lots of helpful reverse-engineering.
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
        for &Mmap { logical, physical } in self.mmap.iter() {
            if logical == 0 || physical == 0 {
                continue;
            }

            let mask = logical.get_bits(0..=13) << 16;
            let virt_addr = logical.get_bits(16..=29) << 16;
            let phys_addr = physical.get_bits(16..=29) << 16;
            let prot = Protection {
                r: physical.get_bit(8),
                w: physical.get_bit(9),
                d: physical.get_bit(10),
                x: physical.get_bit(11),
            };

            // debug!(
            //     "[{:x?}:{:x?}|{:x?}] {:x?} ",
            //     virt_addr, phys_addr, mask, addr
            // );

            // This is how the translation is supposed to work according to MrH's doc.
            // Unfortunately, it doesn't work, and I'm not sure why...
            //
            // let final_addr = {
            //     if (addr & mask) != (virt_addr & mask) {
            //         continue;
            //     }
            //     (addr & !mask) | (phys_addr & mask)
            // };
            //
            // return (final_addr, prot);

            // This other approach is based off some random tidbit of info that hinted the
            // minimum remapable size was 512k. It _also_ doesn't work...
            //
            // if (virt_addr..(virt_addr + mask / 2)).contains(&addr) {
            //     let final_addr = addr - virt_addr + phys_addr;
            //     return (final_addr, prot);
            // }

            // XXX: I've spent _way_ too much time trying to decipher how to
            // mmap properly, so fuck it. I'm just hardcoding the few mappings
            // software uses on a case-by-case basis.
            let transform_range = match (mask, virt_addr, phys_addr) {
                // ipodloader2: map SDRAM to 0x0
                (0x3a00_0000, 0, 0x1000_0000) => Some(0..0x0200_0000),
                // ipodloader2: map flash ROM to 0x2000_0000
                (0x3a00_0000, 0x2000_0000, 0) => Some(0x2000_0000..0x2010_0000),
                // flashROM: flashROM protection bits
                (0x3bf0_0000, 0, 0) => Some(0..0),
                _ => None,
            };

            if let Some(transform_range) = transform_range {
                if transform_range.contains(&addr) {
                    return (addr - virt_addr + phys_addr, prot);
                }
            } else {
                panic!("unimplemented mmap: {:x?}", (mask, virt_addr, phys_addr))
            }
        }

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
            0xf000..=0xf03f if offset & 4 == 0 => {
                let no = (offset - 0xf000) / 8;
                self.mmap[no as usize].logical = val;

                Err(Log(
                    Debug,
                    format!(
                        "virt_addr:{:x}, mask:{:x}",
                        val.get_bits(16..=29) << 16,
                        val.get_bits(0..=13) << 16,
                    ),
                ))
            }
            0xf000..=0xf03f if offset & 4 != 0 => {
                let no = (offset - 0xf000) / 8;
                self.mmap[no as usize].physical = val;

                Err(Log(
                    Debug,
                    format!(
                        "phys_addr:{:x}, rwdx:{:04b}",
                        val.get_bits(16..=29) << 16,
                        val.get_bits(8..=11),
                    ),
                ))
            }
            0xf040 => Err(StubWrite(Info, self.cache_mask = val)),
            0xf044 => Err(StubWrite(Info, ())),
            0xf048 => Err(Unimplemented),
            _ => Err(Unexpected),
        }
    }
}

use crate::devices::prelude::*;
use crate::memory::MemAccessKind;

use super::common::CpuId;

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
#[derive(Debug)]
pub struct MemCon {
    selected: CpuId,
    cpucon: MemConImpl,
    copcon: MemConImpl,
}

impl MemCon {
    pub fn new() -> MemCon {
        MemCon {
            selected: CpuId::Cpu,
            cpucon: MemConImpl::new(),
            copcon: MemConImpl::new(),
        }
    }

    pub fn virt_to_phys(&self, addr: u32, access: MemAccessKind) -> (u32, Protection) {
        match self.selected {
            CpuId::Cpu => self.cpucon.virt_to_phys(addr, access),
            CpuId::Cop => self.copcon.virt_to_phys(addr, access),
        }
    }

    pub fn set_cpuid(&mut self, cpu: CpuId) {
        self.selected = cpu
    }
}

impl Device for MemCon {
    fn kind(&self) -> &'static str {
        "<cpu/cop router>"
    }

    fn probe(&self, offset: u32) -> Probe {
        match self.selected {
            CpuId::Cpu => Probe::from_device(&self.cpucon, offset),
            CpuId::Cop => Probe::from_device(&self.copcon, offset),
        }
    }
}

impl Memory for MemCon {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match self.selected {
            CpuId::Cpu => self.cpucon.r32(offset),
            CpuId::Cop => self.copcon.r32(offset),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match self.selected {
            CpuId::Cpu => self.cpucon.w32(offset, val),
            CpuId::Cop => self.copcon.w32(offset, val),
        }
    }
}

/// PP5020 Memory Controller.
///
/// Shoutout to the mysterious MrH for lots of helpful reverse-engineering.
/// https://daniel.haxx.se/sansa/memory_controller.txt
struct MemConImpl {
    cache_data: Box<[u32; 0x2000]>,
    /// A status word is 32 bits and is mirrored four times for each cache line
    ///
    /// bit 0-20    line_address >> 11
    /// bit 21      unused?
    /// bit 22      line_dirty
    /// bit 23      line_valid
    /// bit 24-31   unused?
    cache_status: Box<[u32; 0x2000]>,

    mmap: [Mmap; 8],
    cache_mask: u32,
    cache_control: u32,
    /// Set back to zero after use
    cache_flush_mask: u32,
}

impl std::fmt::Debug for MemConImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("MemConImpl")
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
            .field("cache_control", &self.cache_control)
            .field("cache_flush_mask", &self.cache_flush_mask)
            .finish()
    }
}

impl MemConImpl {
    pub fn new() -> MemConImpl {
        MemConImpl {
            cache_data: Box::new([0; 0x2000]),
            cache_status: Box::new([0; 0x2000]),
            mmap: Default::default(),
            cache_mask: 0,
            cache_control: 0,
            cache_flush_mask: 0,
        }
    }

    fn virt_to_phys(&self, addr: u32, access: MemAccessKind) -> (u32, Protection) {
        for &Mmap { logical, physical } in self.mmap.iter() {
            if logical == 0 || physical == 0 {
                continue;
            }

            let mask = logical.get_bits(11..=13) << 28;
            let virt_addr = logical.get_bits(16..=31) << 16;
            let phys_addr = physical.get_bits(16..=31) << 16;
            let prot = Protection {
                r: physical.get_bit(8),
                w: physical.get_bit(9),
                d: physical.get_bit(10),
                x: physical.get_bit(11),
            };

            if access == MemAccessKind::Read && !prot.r && !prot.d {
                continue;
            }
            if access == MemAccessKind::Write && !prot.w{
                continue;
            }
            if access == MemAccessKind::Execute && !prot.x {
                continue;
            }

            // This is how the translation is supposed to work according to MrH's doc.
            let final_addr = {
                if (addr & mask) != (virt_addr & mask) {
                    continue;
                }
                (addr & !mask) | (phys_addr & mask)
            };
            
            return (final_addr, prot);
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

impl Device for MemConImpl {
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

impl Memory for MemConImpl {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x0000..=0x1fff => Err(StubRead(Error, self.cache_data[offset as usize])),
            0x2000..=0x3fff => Err(Unimplemented),
            0x4000..=0x5fff => Err(StubRead(
                Error,
                self.cache_status[(offset - 0x4000) as usize],
            )),
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
            0xf044 => Err(StubRead(Info, self.cache_control)),
            0xf048 => Err(Unimplemented),
            _ => Err(Unexpected),
        }
    }

    fn w16(&mut self, offset: u32, val: u16) -> MemResult<()> {
        match offset {
            0xf000..=0xf03f if offset & 4 != 0 => {
                let no = (offset - 0xf000) / 8;
                self.mmap[no as usize].physical &= !0xffff;
                self.mmap[no as usize].physical |= val as u32;

                warn!(
                    target: "MMIO",
                    "phys_addr:{:x}, rwdx:{:04b}",
                    self.mmap[no as usize].physical.get_bits(16..=31) << 16,
                    self.mmap[no as usize].physical.get_bits(8..=11),
                );
                Err(StubWrite(Warn, ()))
            },
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x0000..=0x1fff => Err(StubWrite(Error, self.cache_data[offset as usize] = val)),
            0x2000..=0x3fff => Err(Unimplemented),
            0x4000..=0x5fff => Err(StubWrite(
                Error,
                self.cache_status[(offset - 0x4000) as usize] = val,
            )),
            0x6000..=0x7fff => Err(Unimplemented),
            0x8000..=0x9fff => Err(StubWrite(Info, ())),
            0xa000..=0xbfff => Err(StubWrite(Info, ())),
            0xc000..=0xdfff => Err(StubWrite(Info, ())),
            0xf000..=0xf03f if offset & 4 == 0 => {
                let no = (offset - 0xf000) / 8;
                self.mmap[no as usize].logical = val;
                warn!(
                    target: "MMIO",
                    "virt_addr:{:x}, mask:{:x}",
                    val.get_bits(16..=31) << 16,
                    val.get_bits(0..=16) << 16,
                );

                Err(StubWrite(Warn, ()))
            }
            0xf000..=0xf03f if offset & 4 != 0 => {
                let no = (offset - 0xf000) / 8;
                self.mmap[no as usize].physical = val;

                warn!(
                    target: "MMIO",
                    "phys_addr:{:x}, rwdx:{:04b}",
                    val.get_bits(16..=31) << 16,
                    val.get_bits(8..=11),
                );
                Err(StubWrite(Warn, ()))
            }
            0xf040 => Err(StubWrite(Info, self.cache_mask = val)),
            0xf044 => Err(StubWrite(Info, self.cache_control = val)),
            0xf048 => Err(Unimplemented),
            _ => Err(Unexpected),
        }
    }
}

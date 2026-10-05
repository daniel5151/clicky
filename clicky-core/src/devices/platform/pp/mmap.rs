#[macro_export]
macro_rules! board_mmap {
    (
        $board:ty, $kind:literal, $soc:ident;

        RAM {
            $($start_ram:literal $(..= $end_ram:literal)? => $ram:ident,)*
        }
        DEVICES {
            $($start_dev:literal $(..= $end_dev:literal)? => $dev:ident,)*
        }
    ) => {
        const _: () = {
            use $crate::devices::{Device, Probe};
            use $crate::error::{MemException, MemResult};
            use $crate::memory::{MemAccessKind, Memory};

            macro_rules! impl_mem_r {
                ($fn:ident, $ret:ty) => {
                    fn $fn(&mut self, addr: u32) -> MemResult<$ret> {
                        let mut addr = addr;
                        if (0x00..0x1F).contains(&addr) && self.$soc.cachecon.local_evt {
                            addr |= 0x6000_f100;
                        }

                        let (phys_addr, prot) = self.$soc.memcon.virt_to_phys(addr, MemAccessKind::Read);
                        if !prot.r {
                            return Err(MemException::MmuViolation)
                        }

                        match phys_addr {
                            $($start_ram$(..=$end_ram)? => self.$ram.$fn(phys_addr - $start_ram),)*
                            $($start_dev$(..=$end_dev)? => self.$dev.$fn(phys_addr - $start_dev),)*
                            _ => self.$soc.$fn(phys_addr),
                        }
                    }
                };
            }

            macro_rules! impl_mem_w {
                ($fn:ident, $val:ty) => {
                    fn $fn(&mut self, addr: u32, val: $val) -> MemResult<()> {
                        let (phys_addr, prot) = self.$soc.memcon.virt_to_phys(addr, MemAccessKind::Write);
                        if !prot.w {
                            return Err(MemException::MmuViolation)
                        }

                        match phys_addr {
                            $($start_ram$(..=$end_ram)? => self.$ram.$fn(phys_addr - $start_ram, val),)*
                            $($start_dev$(..=$end_dev)? => self.$dev.$fn(phys_addr - $start_dev, val),)*
                            _ => self.$soc.$fn(phys_addr, val),
                        }
                    }
                };
            }

            macro_rules! impl_mem_x {
                ($fn:ident, $ret:ty) => {
                    fn $fn(&mut self, addr: u32) -> MemResult<$ret> {
                        let phys_addr = if (0x00..0x1F).contains(&addr) && self.$soc.cachecon.local_evt {
                            self.$soc.evp.r32(addr)?
                        } else {
                            let (final_addr, prot) = self.$soc.memcon.virt_to_phys(addr, MemAccessKind::Execute);
                            if !prot.x {
                                return Err(MemException::MmuViolation)
                            }
                            final_addr
                        };

                        match phys_addr {
                            $($start_ram$(..=$end_ram)? => self.$ram.$fn(phys_addr - $start_ram),)*
                            $($start_dev$(..=$end_dev)? => self.$dev.$fn(phys_addr - $start_dev),)*
                            _ => self.$soc.$fn(phys_addr),
                        }
                    }
                };
            }

            impl Device for $board {
                fn kind(&self) -> &'static str {
                    $kind
                }

                fn probe(&self, addr: u32) -> Probe {
                    let (addr, _) = self.$soc.memcon.virt_to_phys(addr, MemAccessKind::Read);
                    match addr {
                        $($start_ram$(..=$end_ram)? => {
                            Probe::from_device(&self.$ram, addr - $start_ram)
                        })*
                        $($start_dev$(..=$end_dev)? => {
                            Probe::from_device(&self.$dev, addr - $start_dev)
                        })*
                        _ => self.$soc.probe(addr),
                    }
                }
            }

            impl Memory for $board {
                impl_mem_r!(r8, u8);
                impl_mem_r!(r16, u16);
                impl_mem_r!(r32, u32);
                impl_mem_w!(w8, u8);
                impl_mem_w!(w16, u16);
                impl_mem_w!(w32, u32);
                impl_mem_x!(x16, u16);
                impl_mem_x!(x32, u32);
            }
        };
    };
}

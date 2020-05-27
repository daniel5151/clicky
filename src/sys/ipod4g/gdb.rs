use armv4t_emu::reg;
use gdbstub::{Access as GdbStubAccess, Target, TargetState};

use super::{BlockMode, Ipod4g, SysError};
use crate::memory::{MemAccess, MemAccessKind, MemAccessVal, Memory};

impl Target for Ipod4g {
    type Usize = u32;
    type Error = SysError;

    fn target_description_xml() -> Option<&'static str> {
        Some(r#"<target version="1.0"><architecture>armv4t</architecture></target>"#)
    }

    fn step(
        &mut self,
        mut log_mem_access: impl FnMut(GdbStubAccess<u32>),
    ) -> Result<TargetState, Self::Error> {
        // transform multi-byte accesses into their constituent
        // single-byte accesses
        let log_access_to_gdb = |access: MemAccess| {
            let mut push = |offset, val| {
                log_mem_access(GdbStubAccess {
                    kind: match access.kind {
                        MemAccessKind::Read => gdbstub::AccessKind::Read,
                        MemAccessKind::Write => gdbstub::AccessKind::Write,
                    },
                    addr: offset,
                    val,
                })
            };

            match access.val {
                MemAccessVal::U8(val) => push(access.offset, val),
                MemAccessVal::U16(val) => val
                    .to_le_bytes()
                    .iter()
                    .enumerate()
                    .for_each(|(i, b)| push(access.offset + i as u32, *b)),
                MemAccessVal::U32(val) => val
                    .to_le_bytes()
                    .iter()
                    .enumerate()
                    .for_each(|(i, b)| push(access.offset + i as u32, *b)),
            }
        };

        if self.step(log_access_to_gdb, BlockMode::NonBlocking)? {
            Ok(TargetState::Running)
        } else {
            Ok(TargetState::Halted)
        }
    }

    // order specified in binutils-gdb/blob/master/gdb/features/arm/arm-core.xml
    fn read_registers(&mut self, mut push_reg: impl FnMut(&[u8])) {
        let mode = self.cpu.mode();
        for i in 0..13 {
            push_reg(&self.cpu.reg_get(mode, i).to_le_bytes());
        }
        push_reg(&self.cpu.reg_get(mode, reg::SP).to_le_bytes()); // 13
        push_reg(&self.cpu.reg_get(mode, reg::LR).to_le_bytes()); // 14
        push_reg(&self.cpu.reg_get(mode, reg::PC).to_le_bytes()); // 15

        // Floating point registers, unused
        for _ in 0..25 {
            push_reg(&[0, 0, 0, 0]);
        }

        push_reg(&self.cpu.reg_get(mode, reg::CPSR).to_le_bytes());
    }

    fn write_registers(&mut self, regs: &[u8]) {
        if regs.len() != (16 + 25 + 1) * 4 {
            error!("Wrong data length for write_registers: {}", regs.len());
            return;
        }
        let mut next = {
            let mut idx: usize = 0;
            move || {
                use std::convert::TryInto;
                idx += 4;
                u32::from_le_bytes(regs[idx - 4..idx].try_into().unwrap())
            }
        };
        let mode = self.cpu.mode();
        for i in 0..13 {
            self.cpu.reg_set(mode, i, next());
        }
        self.cpu.reg_set(mode, reg::SP, next());
        self.cpu.reg_set(mode, reg::LR, next());
        self.cpu.reg_set(mode, reg::PC, next());
        // Floating point registers, unused
        for _ in 0..25 {
            next();
        }

        self.cpu.reg_set(mode, reg::CPSR, next());
    }

    fn read_pc(&mut self) -> u32 {
        self.cpu.reg_get(self.cpu.mode(), reg::PC)
    }

    fn read_addrs(&mut self, addr: std::ops::Range<u32>, mut push_byte: impl FnMut(u8)) {
        for addr in addr {
            match self.devices.r8(addr) {
                Ok(val) => push_byte(val),
                Err(_) => {
                    // the only errors that RAM emits are accessing uninitialized memory, which gdb
                    // will do _a lot_. We'll just squelch these errors...
                    push_byte(0x00)
                }
            };
        }
    }

    fn write_addrs(&mut self, mut get_addr_val: impl FnMut() -> Option<(u32, u8)>) {
        while let Some((addr, val)) = get_addr_val() {
            match self.devices.w8(addr, val) {
                Ok(_) => {}
                Err(e) => warn!("gdbstub write_addrs memory exception: {:?}", e),
            };
        }
    }
}

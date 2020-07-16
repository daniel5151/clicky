use std::collections::HashMap;

use armv4t_emu::reg;
use gdbstub::{
    arch, outputln, BreakOp, ConsoleOutput, OptResult, ResumeAction, StopReason, Target, Tid,
    TidSelector, WatchKind,
};

use super::{BlockMode, CpuId, FatalError, Ipod4g};
use crate::memory::{MemAccessKind, Memory};

pub struct Ipod4gGdb {
    sys: Ipod4g,

    selected_core: CpuId,
    watchpoints: Vec<u32>,
    watchpoint_kinds: HashMap<u32, MemAccessKind>,
    breakpoints: Vec<u32>,
}

impl Ipod4gGdb {
    pub fn new(sys: Ipod4g) -> Ipod4gGdb {
        Ipod4gGdb {
            sys,
            selected_core: CpuId::Cpu,
            watchpoints: Vec::new(),
            watchpoint_kinds: HashMap::new(),
            breakpoints: Vec::new(),
        }
    }

    pub fn sys_ref(&self) -> &Ipod4g {
        &self.sys
    }

    pub fn sys_mut(&mut self) -> &mut Ipod4g {
        &mut self.sys
    }

    fn step(&mut self) -> Result<Option<(Tid, StopReason<u32>)>, FatalError> {
        let mut hit_watchpoint = None;

        let watchpoint_kinds = &self.watchpoint_kinds;
        self.sys.step(
            BlockMode::NonBlocking,
            (&self.watchpoints, |cpuid, access| {
                if watchpoint_kinds.get(&access.offset) == Some(&access.kind) {
                    hit_watchpoint = Some((cpuid, access))
                }
            }),
        )?;

        if let Some((id, access)) = hit_watchpoint {
            let cpu = match id {
                CpuId::Cpu => &mut self.sys.cpu,
                CpuId::Cop => &mut self.sys.cop,
            };

            let pc = cpu.reg_get(cpu.mode(), reg::PC);
            cpu.reg_set(
                cpu.mode(),
                reg::PC,
                pc - if cpu.thumb_mode() { 2 } else { 4 },
            );

            let reason = match access.kind {
                MemAccessKind::Read => StopReason::Watch {
                    kind: WatchKind::Read,
                    addr: access.offset,
                },
                MemAccessKind::Write => StopReason::Watch {
                    kind: WatchKind::Write,
                    addr: access.offset,
                },
            };
            return Ok(Some((cpuid_to_tid(id), reason)));
        }

        for (id, cpu) in &mut [
            (CpuId::Cpu, &mut self.sys.cpu),
            (CpuId::Cop, &mut self.sys.cop),
        ] {
            let pc = cpu.reg_get(cpu.mode(), reg::PC);
            if self.breakpoints.contains(&pc) {
                return Ok(Some((cpuid_to_tid(*id), StopReason::SwBreak)));
            }
        }

        Ok(None)
    }
}

fn cpuid_to_tid(id: CpuId) -> Tid {
    match id {
        CpuId::Cpu => Tid::new(1).unwrap(),
        CpuId::Cop => Tid::new(2).unwrap(),
    }
}

impl Target for Ipod4gGdb {
    type Arch = arch::arm::Armv4t;
    type Error = FatalError;

    fn resume(
        &mut self,
        actions: &mut dyn Iterator<Item = (TidSelector, ResumeAction)>,
        check_gdb_interrupt: &mut dyn FnMut() -> bool,
    ) -> Result<(Tid, StopReason<u32>), Self::Error> {
        let action = actions.next().unwrap().1;
        // FIXME: support cases where there is more than one action
        assert!(actions.next().is_none());

        match action {
            ResumeAction::Step => match self.step()? {
                Some(stop_reason) => Ok(stop_reason),
                None => Ok((cpuid_to_tid(self.selected_core), StopReason::DoneStep)),
            },
            ResumeAction::Continue => {
                let mut cycles: usize = 0;
                loop {
                    // check for GDB interrupt every 1024 instructions
                    if cycles % 1024 == 0 && check_gdb_interrupt() {
                        break Ok((cpuid_to_tid(self.selected_core), StopReason::GdbInterrupt));
                    }
                    cycles += 1;

                    if let Some(stop_reason) = self.step()? {
                        break Ok(stop_reason);
                    };
                }
            }
        }
    }

    fn read_registers(&mut self, regs: &mut arch::arm::reg::ArmCoreRegs) -> Result<(), FatalError> {
        let cpu = match self.selected_core {
            CpuId::Cpu => &mut self.sys.cpu,
            CpuId::Cop => &mut self.sys.cop,
        };

        let mode = cpu.mode();

        for i in 0..13 {
            regs.r[i] = cpu.reg_get(mode, i as u8);
        }
        regs.sp = cpu.reg_get(mode, reg::SP);
        regs.lr = cpu.reg_get(mode, reg::LR);
        regs.pc = cpu.reg_get(mode, reg::PC);
        regs.cpsr = cpu.reg_get(mode, reg::CPSR);

        Ok(())
    }

    fn write_registers(&mut self, regs: &arch::arm::reg::ArmCoreRegs) -> Result<(), FatalError> {
        let cpu = match self.selected_core {
            CpuId::Cpu => &mut self.sys.cpu,
            CpuId::Cop => &mut self.sys.cop,
        };

        let mode = cpu.mode();

        for i in 0..13 {
            cpu.reg_set(mode, i, regs.r[i as usize]);
        }
        cpu.reg_set(mode, reg::SP, regs.sp);
        cpu.reg_set(mode, reg::LR, regs.lr);
        cpu.reg_set(mode, reg::PC, regs.pc);
        cpu.reg_set(mode, reg::CPSR, regs.cpsr);

        Ok(())
    }

    fn read_addrs(
        &mut self,
        addr: std::ops::Range<u32>,
        push_byte: &mut dyn FnMut(u8),
    ) -> Result<(), FatalError> {
        for addr in addr {
            match self.sys.devices.r8(addr) {
                Ok(b) => push_byte(b),
                // the only errors that RAM emits are accessing uninitialized memory, which gdb
                // will do _a lot_. We'll just squelch these errors...
                Err(_) => push_byte(0x00),
            }
        }
        Ok(())
    }

    fn write_addrs(&mut self, start_addr: u32, data: &[u8]) -> Result<(), FatalError> {
        for (addr, val) in (start_addr..).zip(data.iter().copied()) {
            match self.sys.devices.w8(addr, val) {
                Ok(_) => {}
                Err(e) => warn!("gdbstub write_addrs memory exception: {:?}", e),
            };
        }
        Ok(())
    }

    fn update_sw_breakpoint(&mut self, addr: u32, op: BreakOp) -> Result<bool, FatalError> {
        match op {
            BreakOp::Add => self.breakpoints.push(addr),
            BreakOp::Remove => {
                let pos = match self.breakpoints.iter().position(|x| *x == addr) {
                    None => return Ok(false),
                    Some(pos) => pos,
                };
                self.breakpoints.remove(pos);
            }
        }

        Ok(true)
    }

    fn update_hw_watchpoint(
        &mut self,
        addr: u32,
        op: BreakOp,
        kind: WatchKind,
    ) -> OptResult<bool, FatalError> {
        match op {
            BreakOp::Add => {
                let access_kind = match kind {
                    WatchKind::Write => MemAccessKind::Write,
                    WatchKind::Read => MemAccessKind::Read,
                    // FIXME: properly support ReadWrite breakpoints
                    WatchKind::ReadWrite => MemAccessKind::Read,
                };
                self.watchpoints.push(addr);
                self.watchpoint_kinds.insert(addr, access_kind);
            }
            BreakOp::Remove => {
                let pos = match self.watchpoints.iter().position(|x| *x == addr) {
                    None => return Ok(false),
                    Some(pos) => pos,
                };
                self.watchpoints.remove(pos);
                self.watchpoint_kinds.remove(&addr);
            }
        }

        Ok(true)
    }

    fn handle_monitor_cmd(
        &mut self,
        cmd: &[u8],
        mut out: ConsoleOutput,
    ) -> OptResult<(), Self::Error> {
        let cmd = match core::str::from_utf8(cmd) {
            Ok(cmd) => cmd,
            Err(_) => {
                outputln!(out, "command must be valid UTF-8");
                return Ok(());
            }
        };

        match cmd {
            "" => outputln!(out, "Use `monitor help` to list all available commands."),
            "dumpsys" => outputln!(out, "{:#?}", self.sys),
            "help" => {
                outputln!(out, "Available commands:");
                outputln!(out, "-------------------");
                outputln!(out, "  dumpsys - pretty-print a debug view of the system");
                outputln!(out, "  help    - show this help message");
            }
            _ => {
                outputln!(out, "Unsupported command '{}'.", cmd);
                outputln!(out, "Use `monitor help` to list all available commands.");
            }
        }

        Ok(())
    }

    fn list_active_threads(
        &mut self,
        register_thread: &mut dyn FnMut(Tid),
    ) -> Result<(), Self::Error> {
        register_thread(cpuid_to_tid(CpuId::Cpu));
        register_thread(cpuid_to_tid(CpuId::Cop));
        Ok(())
    }

    fn set_current_thread(&mut self, tid: Tid) -> OptResult<(), Self::Error> {
        match tid.get() {
            1 => self.selected_core = CpuId::Cpu,
            2 => self.selected_core = CpuId::Cop,
            _ => unreachable!(),
        }
        Ok(())
    }
}

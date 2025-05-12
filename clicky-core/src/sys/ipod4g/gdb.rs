use std::collections::HashMap;

use armv4t_emu::reg;
use gdbstub_arch;
use gdbstub::common::Tid;
use gdbstub::target;
use gdbstub::target::ext::base::GdbInterrupt;
use gdbstub::target::ext::base::multithread::{
    MultiThreadOps, ResumeAction, ThreadStopReason,
};
use gdbstub::target::ext::breakpoints::WatchKind;
use gdbstub::target::ext::monitor_cmd::{outputln, ConsoleOutput};
use gdbstub::target::{Target, TargetResult};

use crate::devices::Device;
use crate::error::*;
use crate::memory::{MemAccess, MemAccessKind, MemAccessVal, Memory};

use super::{BlockMode, CpuId, Ipod4g};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Event {
    Break,
    WatchWrite(u32),
    WatchRead(u32),
}

pub struct Ipod4gGdb {
    sys: Ipod4g,

    watchpoints: Vec<u32>,
    watchpoint_kinds: HashMap<u32, MemAccessKind>,
    breakpoints: Vec<u32>,

    single_step_irq: bool,
    resume_action_is_step: Option<bool>,
}

impl Ipod4gGdb {
    pub fn new(sys: Ipod4g) -> Ipod4gGdb {
        Ipod4gGdb {
            sys,
            watchpoints: Vec::new(),
            watchpoint_kinds: HashMap::new(),
            breakpoints: Vec::new(),
            single_step_irq: false,
            resume_action_is_step: None,
        }
    }

    pub fn sys_ref(&self) -> &Ipod4g {
        &self.sys
    }

    pub fn sys_mut(&mut self) -> &mut Ipod4g {
        &mut self.sys
    }

    fn step(&mut self) -> Result<Option<(Event, CpuId)>, FatalMemException> {
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

            return Ok(Some((
                match access.kind {
                    MemAccessKind::Read => Event::WatchRead(access.offset),
                    MemAccessKind::Write => Event::WatchWrite(access.offset),
                    MemAccessKind::Execute => Event::WatchRead(access.offset),
                },
                id,
            )));
        }

        for (id, cpu) in &mut [
            (CpuId::Cpu, &mut self.sys.cpu),
            (CpuId::Cop, &mut self.sys.cop),
        ] {
            let pc = cpu.reg_get(cpu.mode(), reg::PC);
            if self.breakpoints.contains(&pc) {
                return Ok(Some((Event::Break, *id)));
            }
        }

        Ok(None)
    }

    fn exec_dbg_command(&mut self, cmd: &str, out: &mut ConsoleOutput) -> Result<(), String> {
        const HELP: &str = r#"
Available commands:
===================

Query System State
--------------------------------------------------------------------------------
  dumpsys            - pretty-print a debug view of the system
  probe <addr>       - probe what device is at the specified address

Debugging
--------------------------------------------------------------------------------
  single_step_irq <bool> - enable IRQs while single-stepping (default: false)

    Setting this to "false" will artificially prevent devices from generating
    IRQs while single-step debugging. This is great for debugging most
    user-level code, but should be turned off when debugging working with
    low-level, IRQ sensitive code (e.g: early boot, context switching, etc...)

Help
--------------------------------------------------------------------------------
  help               - show this help message
"#;

        let mut s = cmd.split(' ');
        let cmd = match s.next() {
            None | Some("") => {
                outputln!(out, "Use `monitor help` to list all available commands.");
                return Ok(());
            }
            Some(cmd) => cmd,
        };

        match cmd {
            "help" => outputln!(out, "{}", HELP),
            "dumpsys" => outputln!(out, "{:#x?}", self.sys),
            "probe" => {
                let addr = s.next().ok_or("no addr provided")?;
                let addr = match addr.as_bytes() {
                    [b'0', b'x', ..] => u32::from_str_radix(addr.trim_start_matches("0x"), 16),
                    _ => addr.parse::<u32>(),
                }
                .map_err(|_| "couldn't parse addr")?;

                outputln!(out, "{}", self.sys.devices.probe(addr))
            }
            "single_step_irq" => {
                match s.next() {
                    None => {}
                    Some(toggle_s) => {
                        self.single_step_irq = toggle_s
                            .parse::<bool>()
                            .map_err(|_| "couldn't parse <bool>")?
                    }
                };

                outputln!(out, "single_step_irq = {}", self.single_step_irq)
            }
            _ => {
                return Err(format!(
                    "Unsupported command '{}'.\nUse `monitor help` to list all available commands.",
                    cmd
                ))
            }
        }

        Ok(())
    }
}

fn cpuid_to_tid(id: CpuId) -> Tid {
    match id {
        CpuId::Cpu => Tid::new(1).unwrap(),
        CpuId::Cop => Tid::new(2).unwrap(),
    }
}

fn tid_to_cpuid(tid: Tid) -> Option<CpuId> {
    Some(match tid.get() {
        1 => CpuId::Cpu,
        2 => CpuId::Cop,
        _ => return None,
    })
}

fn event_to_stopreason(e: Event, id: CpuId) -> ThreadStopReason<u32> {
    let tid = cpuid_to_tid(id);
    match e {
        Event::Break => ThreadStopReason::SwBreak(tid),
        Event::WatchWrite(addr) => ThreadStopReason::Watch {
            tid,
            kind: WatchKind::Write,
            addr,
        },
        Event::WatchRead(addr) => ThreadStopReason::Watch {
            tid,
            kind: WatchKind::Read,
            addr,
        },
    }
}

impl Target for Ipod4gGdb {
    type Arch = gdbstub_arch::arm::Armv4t;
    type Error = FatalMemException;

    fn base_ops(&mut self) -> target::ext::base::BaseOps<Self::Arch, Self::Error> {
        target::ext::base::BaseOps::MultiThread(self)
    }

    fn breakpoints(&mut self) -> Option<target::ext::breakpoints::BreakpointsOps<Self>> {
        Some(self)
    }

    fn monitor_cmd(&mut self) -> Option<target::ext::monitor_cmd::MonitorCmdOps<Self>> {
        Some(self)
    }
}

impl target::ext::breakpoints::Breakpoints for Ipod4gGdb {
    fn sw_breakpoint(&mut self) -> Option<target::ext::breakpoints::SwBreakpointOps<Self>> {
        Some(self)
    }

    fn hw_watchpoint(&mut self) -> Option<target::ext::breakpoints::HwWatchpointOps<Self>> {
        Some(self)
    }
}

impl MultiThreadOps for Ipod4gGdb {
    fn resume(
        &mut self,
        default_resume_action: ResumeAction,
        gdb_interrupt: GdbInterrupt<'_>,
    ) -> Result<ThreadStopReason<u32>, Self::Error> {
        let default_resume_action_is_step = match default_resume_action {
            ResumeAction::Step => true,
            ResumeAction::Continue => false,
            _ => return Ok(ThreadStopReason::DoneStep), // should be an error
        };

        match self
            .resume_action_is_step
            .unwrap_or(default_resume_action_is_step)
        {
            true => {
                if !self.single_step_irq {
                    self.sys.skip_irq_check = true;
                }
                let res = match self.step()? {
                    Some((event, cpuid)) => Ok(event_to_stopreason(event, cpuid)),
                    None => Ok(ThreadStopReason::DoneStep),
                };
                if !self.single_step_irq {
                    self.sys.skip_irq_check = false;
                }
                res
            },
            false => {
                let mut gdb_interrupt = gdb_interrupt.no_async();
                let mut cycles: usize = 0;
                loop {
                    // check for GDB interrupt every 1024 instructions
                    if cycles % 1024 == 0 && gdb_interrupt.pending() {
                        return Ok(ThreadStopReason::GdbInterrupt);
                    }
                    cycles += 1;

                    if let Some((event, cpuid)) = self.step()? {
                        return Ok(event_to_stopreason(event, cpuid));
                    };
                }
            }
        }
    }

    fn clear_resume_actions(&mut self) -> Result<(), Self::Error> {
        self.resume_action_is_step = None;
        Ok(())
    }

    fn set_resume_action(&mut self, tid: Tid, action: ResumeAction) -> Result<(), Self::Error> {
        // in this emulator, each core runs in lock-step, so we don't actually care
        // about the specific tid. In real integrations, you very much should!

        if self.resume_action_is_step.is_some() {
            return Ok(());
        }

        let cpu = match tid_to_cpuid(tid).unwrap() {
            CpuId::Cpu => &mut self.sys.cpu,
            CpuId::Cop => &mut self.sys.cop,
        };

        self.resume_action_is_step = match action {
            ResumeAction::Step => Some(true),
            ResumeAction::Continue => Some(false),
            _ => return MemException::Fatal("no support for resuming with signal".to_owned()).resolve(
                "Debug",
                MemExceptionCtx {
                    pc: cpu.reg_get(cpu.mode(), reg::PC),
                    access: MemAccess {
                        kind: MemAccessKind::Read,
                        offset: 0,
                        val: MemAccessVal::U8(0),
                    },
                    in_device: format!("{}", tid_to_cpuid(tid).unwrap()),
                },
            ),
        };

        Ok(())
    }

    fn read_registers(
        &mut self,
        regs: &mut gdbstub_arch::arm::reg::ArmCoreRegs,
        tid: Tid,
    ) -> TargetResult<(), Self> {
        let cpu = match tid_to_cpuid(tid).unwrap() {
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

    fn write_registers(
        &mut self,
        regs: &gdbstub_arch::arm::reg::ArmCoreRegs,
        tid: Tid,
    ) -> TargetResult<(), Self> {
        let cpu = match tid_to_cpuid(tid).unwrap() {
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

    fn read_addrs(&mut self, start_addr: u32, data: &mut [u8], tid: Tid) -> TargetResult<(), Self> {
        self.sys.devices.cpuid.set_cpuid(tid_to_cpuid(tid).unwrap());
        self.sys
            .devices
            .memcon
            .set_cpuid(tid_to_cpuid(tid).unwrap());

        for (addr, val) in (start_addr..).zip(data.iter_mut()) {
            // TODO: throw a fatal error when accessing non-RAM devices?
            *val = self.sys.devices.r8(addr).map_err(drop)?
        }
        Ok(())
    }

    fn write_addrs(&mut self, start_addr: u32, data: &[u8], tid: Tid) -> TargetResult<(), Self> {
        self.sys.devices.cpuid.set_cpuid(tid_to_cpuid(tid).unwrap());
        self.sys
            .devices
            .memcon
            .set_cpuid(tid_to_cpuid(tid).unwrap());

        for (addr, val) in (start_addr..).zip(data.iter().copied()) {
            // TODO: throw a fatal error when accessing non-RAM devices?
            self.sys.devices.w8(addr, val).map_err(drop)?
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
}

impl target::ext::breakpoints::SwBreakpoint for Ipod4gGdb {
    fn add_sw_breakpoint(&mut self, addr: u32, _kind: gdbstub_arch::arm::ArmBreakpointKind) -> TargetResult<bool, Self> {
        self.breakpoints.push(addr);
        Ok(true)
    }

    fn remove_sw_breakpoint(&mut self, addr: u32, _kind: gdbstub_arch::arm::ArmBreakpointKind) -> TargetResult<bool, Self> {
        match self.breakpoints.iter().position(|x| *x == addr) {
            None => return Ok(false),
            Some(pos) => self.breakpoints.remove(pos),
        };

        Ok(true)
    }
}

// FIXME: this watchpoint implementation could probably use some work.

impl target::ext::breakpoints::HwWatchpoint for Ipod4gGdb {
    fn add_hw_watchpoint(&mut self, addr: u32, kind: WatchKind) -> TargetResult<bool, Self> {
        let access_kind = match kind {
            WatchKind::Write => MemAccessKind::Write,
            WatchKind::Read => MemAccessKind::Read,
            // FIXME: properly support ReadWrite breakpoints
            WatchKind::ReadWrite => MemAccessKind::Read,
        };
        self.watchpoints.push(addr);
        self.watchpoint_kinds.insert(addr, access_kind);

        Ok(true)
    }

    fn remove_hw_watchpoint(&mut self, addr: u32, _kind: WatchKind) -> TargetResult<bool, Self> {
        let pos = match self.watchpoints.iter().position(|x| *x == addr) {
            None => return Ok(false),
            Some(pos) => pos,
        };
        self.watchpoints.remove(pos);
        self.watchpoint_kinds.remove(&addr);

        Ok(true)
    }
}

impl target::ext::monitor_cmd::MonitorCmd for Ipod4gGdb {
    fn handle_monitor_cmd(
        &mut self,
        cmd: &[u8],
        mut out: ConsoleOutput,
    ) -> Result<(), Self::Error> {
        let cmd = match core::str::from_utf8(cmd) {
            Ok(s) => s,
            Err(_) => {
                outputln!(out, "command must be valid UTF-8");
                return Ok(());
            }
        };

        if let Err(e) = self.exec_dbg_command(cmd, &mut out) {
            outputln!(out, "ERROR: {}", e)
        }

        Ok(())
    }
}

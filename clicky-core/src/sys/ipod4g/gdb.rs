use std::collections::HashMap;

use armv4t_emu::reg;
use gdbstub::common::Signal;
use gdbstub::common::Tid;
use gdbstub::target;
use gdbstub::stub::MultiThreadStopReason;
use gdbstub::target::ext::base::multithread::{MultiThreadBase, MultiThreadSingleStep, MultiThreadResume};
use gdbstub::target::ext::breakpoints::WatchKind;
use gdbstub::target::ext::monitor_cmd::{outputln, ConsoleOutput};
use gdbstub::target::{Target, TargetResult};
use gdbstub_arch;

use crate::devices::Device;
use crate::error::*;
use crate::memory::{MemAccessKind, Memory};

use super::{BlockMode, CpuId, Ipod4g};

#[derive(Hash)]
pub enum ExecMode {
    Step,
    Continue,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Event {
    Break,
    WatchWrite(u32),
    WatchRead(u32),
}

pub struct Ipod4gGdb {
    sys: Ipod4g,

    exec_mode: HashMap<CpuId, ExecMode>,

    watchpoints: Vec<u32>,
    watchpoint_kinds: HashMap<u32, MemAccessKind>,
    breakpoints: Vec<u32>,

    single_step_irq: bool,
}

impl Ipod4gGdb {
    pub fn new(sys: Ipod4g) -> Ipod4gGdb {
        Ipod4gGdb {
            sys,
            exec_mode: HashMap::new(),
            watchpoints: Vec::new(),
            watchpoint_kinds: HashMap::new(),
            breakpoints: Vec::new(),
            single_step_irq: false,
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

fn event_to_stopreason(e: Event, id: CpuId) -> MultiThreadStopReason<u32> {
    let tid = cpuid_to_tid(id);
    match e {
        Event::Break => MultiThreadStopReason::SwBreak(tid),
        Event::WatchWrite(addr) => MultiThreadStopReason::Watch {
            tid,
            kind: WatchKind::Write,
            addr,
        },
        Event::WatchRead(addr) => MultiThreadStopReason::Watch {
            tid,
            kind: WatchKind::Read,
            addr,
        },
    }
}

impl Target for Ipod4gGdb {
    type Arch = gdbstub_arch::arm::Armv4t;
    type Error = FatalMemException;

    #[inline(always)]
    fn base_ops(&mut self) -> target::ext::base::BaseOps<'_, Self::Arch, Self::Error> {
        target::ext::base::BaseOps::MultiThread(self)
    }

    #[inline(always)]
    fn support_breakpoints(
        &mut self,
    ) -> Option<target::ext::breakpoints::BreakpointsOps<'_, Self>> {
        Some(self)
    }
}

impl target::ext::breakpoints::Breakpoints for Ipod4gGdb {
    fn support_sw_breakpoint(
        &mut self,
    ) -> Option<target::ext::breakpoints::SwBreakpointOps<'_, Self>> {
        Some(self)
    }

    fn support_hw_watchpoint(
        &mut self,
    ) -> Option<target::ext::breakpoints::HwWatchpointOps<'_, Self>> {
        Some(self)
    }
}

impl MultiThreadResume for Ipod4gGdb {
    fn resume(&mut self) -> Result<(), Self::Error> {
        // FIXME: properly handle multiple actions...

        let should_single_step = matches!(
            self.exec_mode
                .get(&CpuId::Cpu)
                .or_else(|| self.exec_mode.get(&CpuId::Cop)),
            Some(&ExecMode::Step)
        );

        match should_single_step {
            true => {
                if !self.single_step_irq {
                    self.sys.skip_irq_check = true;
                }
                let res = match self.step()? {
                    Some((event, cpuid)) => Ok(()),
                    None => Ok(()),
                };
                if !self.single_step_irq {
                    self.sys.skip_irq_check = false;
                }
                res
            }
            false => {
                let mut cycles: usize = 0;
                loop {
                    // check for GDB interrupt every 1024 instructions
                    if cycles % 1024 == 0 {
                    //if cycles % 1024 == 0 && check_gdb_interrupt() {
                        return Ok(());
                    }
                    cycles += 1;

                    if let Some((event, cpuid)) = self.step()? {
                        return Ok(());
                    };
                }
            }
        }
    }

    fn clear_resume_actions(&mut self) -> Result<(), Self::Error> {
        self.exec_mode.clear();
        Ok(())
    }

    #[inline(always)]
    fn support_single_step(
        &mut self,
    ) -> Option<target::ext::base::multithread::MultiThreadSingleStepOps<'_, Self>> {
        Some(self)
    }

    fn set_resume_action_continue(
        &mut self,
        tid: Tid,
        signal: Option<Signal>,
    ) -> Result<(), Self::Error> {
        if signal.is_some() {
            panic!("no support for continuing with signal");
        }

        self.exec_mode
            .insert(tid_to_cpuid(tid).expect("AJAJAJ"), ExecMode::Continue);

        Ok(())
    }
}

impl MultiThreadSingleStep for Ipod4gGdb {
    fn set_resume_action_step(
        &mut self,
        tid: Tid,
        signal: Option<Signal>,
    ) -> Result<(), Self::Error> {
        if signal.is_some() {
            panic!("no support for continuing with signal");
        }

        self.exec_mode.insert(tid_to_cpuid(tid).expect("AJAJAJ"), ExecMode::Step);

        Ok(())
    }
}

impl MultiThreadBase for Ipod4gGdb {
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

    fn read_addrs(&mut self, start_addr: u32, data: &mut [u8], tid: Tid) -> TargetResult<usize, Self> {
        self.sys.devices.cpuid.set_cpuid(tid_to_cpuid(tid).unwrap());
        self.sys
            .devices
            .memcon
            .set_cpuid(tid_to_cpuid(tid).unwrap());

        for (addr, val) in (start_addr..).zip(data.iter_mut()) {
            // TODO: throw a fatal error when accessing non-RAM devices?
            *val = self.sys.devices.r8(addr).map_err(drop)?
        }
        Ok(data.len())
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
    fn add_hw_watchpoint(&mut self, addr: u32, _len: u32, kind: WatchKind) -> TargetResult<bool, Self> {
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

    fn remove_hw_watchpoint(&mut self, addr: u32, _len: u32, _kind: WatchKind) -> TargetResult<bool, Self> {
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

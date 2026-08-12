// FireWire controller for PP5020. Interfaced to Texas Instruments TSB43AA82 PHY.
// This peripheral differs from the OG (PP5002) FireWire peripheral as described
// in iPodLinux sources (mach-ipod/tsb43aa82.{c,h}): base address is different,
// and register mapping is also very different.
//
// My current understand of the layout for PP5020 is:
// - +0x000..+0x08b - PP5020 glue registers (undocumented)
// - +0x08c..+0x188 - TSB43AA82 CFR (CFR 00h lands at +0x08c)
// - +0x100..+0x1ff - overlaps the above; see the descriptor-array note below
//
// But...
// - +0x100..+0x1ff looks like an array of 8 descriptors on a 0x20 stride, not
//   CFR: one routine (pc 0x000c0924 / 0x000c0930) writes `+0x0c` then `+0x00` of
//   entries 4, 6 and 7 during a RetailOS boot. This overlaps where the CFR
//   would end (+0x188), so the aperture is presumably not just the CFR.
// - +0x050 takes single-bit writes confined to bits 16..22, each followed by a
//   read-back. Under this base it is PP glue, not CFR 50h Agent Control.
// - +0x080/+0x084/+0x088 get written, but map to the read-only ARF/MRF/CRF data
//   read ports.
//
// So in the end this driver is the bare minimum to get into RetailOS/diags,
// which is not really an issue since it would be too much trouble forwarding the
// FireWire trafic to the host.

use crate::devices::prelude::*;

/// Window offset at which the TSB43AA82 CFR appears to start.
const CFR_BASE: u32 = 0x08c;
/// The CFR itself is 0x100 bytes (00h..FCh).
const CFR_LEN: u32 = 0x100;

#[derive(Debug)]
enum Endianness {
    Little,
    Big,
}

#[derive(Debug)]
pub struct Firewire {
    phy_ctrl: u32,
    ttcr: u32,

    /// The observed window is 0x200 bytes, addressed 1:1.
    reg: Box<[u32; 0x80]>,

    endianness: Endianness,
}

/// PHY access register (CFR 20h, i.e. window offset 0xac).
///
/// Bit positions below are in the ARM's view (i.e. already mirrored out of the
/// datasheet's MSB-first numbering). Sec 3.4.8.
///
/// NOTE: there is no "done" flag. Datasheet bits 16-19 are reserved. `RdPy` /
/// `WrPy` are self-clearing once the request has been sent, and the result
/// lands in `RX_ADDR` / `RX_DATA`.
#[allow(dead_code)]
mod phyaccess {
    use std::ops::RangeInclusive;

    /// Read PHY register. Cleared once the request is sent.
    pub const RD_PHY: usize = 31;
    /// Write PHY register. Cleared once the request is sent.
    pub const WR_PHY: usize = 30;
    /// PHY register to access.
    pub const ADDR: RangeInclusive<usize> = 24..=27;
    /// Value to write to the PHY.
    pub const DATA: RangeInclusive<usize> = 16..=23;
    /// Address of the PHY register `RX_DATA` came from.
    pub const RX_ADDR: RangeInclusive<usize> = 8..=11;
    /// Value read back from the PHY.
    pub const RX_DATA: RangeInclusive<usize> = 0..=7;
}

/// CFR 00h reads back as this in little endian mode (sec 3.4.1).
const VERSION_CHIP_ID_LE: u32 = 0x0382_0043;
/// ...and byte-reversed in big endian mode. `LYNX_VERSION_CHIP_ID`.
const VERSION_CHIP_ID_BE: u32 = 0x4300_8203;

/// Name of a CFR register, per the datasheet / iPodLinux's `tsb43aa82.h`.
fn cfr_name(cfr: u32) -> &'static str {
    match cfr {
        0x00 => "CFR:Version",
        0x04 => "CFR:Misc",
        0x08 => "CFR:Ctrl",
        0x0c => "CFR:Interrupt",
        0x10 => "CFR:IMask",
        0x14 => "CFR:CycleTimer",
        0x18 => "CFR:Diagnostic",
        0x20 => "CFR:PhyAccess",
        0x24 => "CFR:BusReset",
        0x28 => "CFR:TimeLimit",
        0x2c => "CFR:AtfStatus",
        0x30 => "CFR:ArfStatus",
        0x34 => "CFR:MtqStatus",
        0x38 => "CFR:MrfStatus",
        0x3c => "CFR:CtqStatus",
        0x40 => "CFR:CrfStatus",
        0x44 => "CFR:OrbFetchCtrl",
        0x48 => "CFR:MgmtAgent",
        0x4c => "CFR:CmdAgent",
        0x50 => "CFR:AgentCtrl",
        0x54 => "CFR:OrbPtr1",
        0x58 => "CFR:OrbPtr2",
        0x5c => "CFR:AgentStatus",
        0x60 => "CFR:TxTimerCtrl",
        0x64 => "CFR:TxTimerStat1",
        0x68 => "CFR:TxTimerStat2",
        0x6c => "CFR:TxTimerStat3",
        0x70 => "CFR:WriteFirst",
        0x74 => "CFR:WriteContinue",
        0x78 => "CFR:WriteUpdate",
        0x80 => "CFR:ArfData",
        0x84 => "CFR:MrfData",
        0x88 => "CFR:CrfData",
        0x8c => "CFR:ConfigRomCtrl",
        0x90 => "CFR:DmaCtrl",
        0x94 => "CFR:BiCtrl",
        0x98 => "CFR:DxfSize",
        0x9c => "CFR:DxfAvail",
        0xa0 => "CFR:DxfAck",
        0xa4 => "CFR:DtfFirstContinue",
        0xa8 => "CFR:DtfUpdate",
        0xac => "CFR:DrfData",
        0xb0..=0xbc => "CFR:DtfCtrl",
        0xc0..=0xcc => "CFR:DrfCtrl",
        0xd0..=0xdc => "CFR:DrfHdr",
        0xe0 => "CFR:DrfTrailer",
        0xe4 => "CFR:DxfPageCount",
        0xe8..=0xf4 => "CFR:DxfHdrStat",
        0xfc => "CFR:LogRomData",
        _ => "CFR:?",
    }
}

impl Firewire {
    pub fn new() -> Firewire {
        Firewire {
            phy_ctrl: 0,
            // CFR 60h power-on default (sec 3.4.24). This is load-bearing, not
            // cosmetic: diagnostics spins forever at pc 0x1000bfb0 waiting on
            // DTTxEd (datasheet bit 0 -> ARM bit 31). Seeding 0 hangs the boot;
            // seeding 0x8000_0000 alone is enough to clear the poll.
            ttcr: 0xFA00_0000,
            reg: Box::new([0; 0x80]),
            endianness: Endianness::Little,
        }
    }

    /// Read a PHY register.
    ///
    /// There's no FireWire bus attached to the emulated iPod, so there's
    /// nothing meaningful to report. Returning zero is the honest answer for a
    /// disconnected port -- but note it *is* a guess that zero is what real
    /// silicon gives for each register, rather than something documented.
    #[allow(dead_code)]
    fn read_phy(&self, _addr: u8) -> u8 {
        0
    }
}

impl Device for Firewire {
    fn kind(&self) -> &'static str {
        "Firewire"
    }

    fn probe(&self, offset: u32) -> Probe {
        // Under the +0x08c hypothesis, anything in the CFR aperture gets a real
        // name. Everything else is presumed to be PP5020 glue, for which there
        // is no documentation.
        let reg = match offset {
            _ if (CFR_BASE..CFR_BASE + CFR_LEN).contains(&offset) => cfr_name(offset - CFR_BASE),
            0x00 => "PP:Version?",
            0x08 => "PP:Ctrl?",
            0x0c => "PP:Interrupt?",
            0x10 => "PP:IMask?",
            0x50 => "PP:Reset?",
            _ => "PP:?",
        };

        Probe::Register(reg)
    }
}

impl Memory for Firewire {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        if offset & 0b11 != 0 {
            return Err(Misaligned);
        }

        let idx = (offset / 4) as usize;
        if idx >= self.reg.len() {
            return Err(Unexpected);
        }

        let val = match offset {
            0x50 => self.phy_ctrl,
            // CFR 00h. The endianness magic is symmetric under any byte
            // permutation, so the *write* can't tell us whether the aperture is
            // byte-swapped -- but this readback can, if the firmware checks it.
            0x8c => match self.endianness {
                Endianness::Little => VERSION_CHIP_ID_LE,
                Endianness::Big => VERSION_CHIP_ID_BE,
            },
            // CFR 60h TxTimer Control
            0xec => self.ttcr,
            _ => self.reg[idx],
        };

        Err(StubRead(Info, val))
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        if offset & 0b11 != 0 {
            return Err(Misaligned);
        }

        let idx = (offset / 4) as usize;
        if idx >= self.reg.len() {
            return Err(Unexpected);
        }

        self.reg[idx] = val;

        match offset {
            0x50 => self.phy_ctrl = val, // iPodLinux says its for reset

            // CFR 00h Version/Revision -- doubles as the endianness control.
            0x8c => match val {
                0x0000_0000 => self.endianness = Endianness::Little,
                0xffff_ffff => self.endianness = Endianness::Big,
                _ => {}
            },

            // CFR 60h TxTimer Control
            0xec => self.ttcr = val,

            _ => {}
        }

        Err(StubWrite(Info, ()))
    }
}

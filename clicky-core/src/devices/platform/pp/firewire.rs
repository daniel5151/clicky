// FireWire controller for PP5020. Unlike PP5002, PP5020 features its own internal
// FireWire peripheral.
//
// Memory mapping is quite close to OHCI, albeit with some usage in a reserved zone.
// Offsets and bit names below follow the 1394 Open Host Controller Interface
// spec; Linux's `drivers/ieee1394/ohci1394.h` is a convenient cross-reference.
// Two values are byte-for-byte what Linux's own driver writes: LinkControlSet
// gets 0x0030_0000 then 0x0000_0200 (ohci1394.c:530 and :541), and the async
// contexts are started by writing the run bit to ContextControlSet.
//
// Only the registers RetailOS and the retail diagnostics actually touch are
// modelled here. Everything else in the window reads back whatever was last
// written to it. There's no FireWire bus attached to the emulated iPod, so this
// only ever needs to be enough to get through boot.
//
// The one range that does *not* fit OHCI is +0x174..+0x17c, which the spec
// leaves reserved. +0x178 gets a 16-byte aligned pointer into the same region
// as the self-ID buffer and the DMA descriptors, so it's presumably a
// PortalPlayer extension.

use crate::devices::prelude::*;

mod reg {
    pub const AT_RETRIES: u32 = 0x008;
    pub const HC_CONTROL_SET: u32 = 0x050;
    pub const HC_CONTROL_CLEAR: u32 = 0x054;
    pub const SELF_ID_BUFFER: u32 = 0x064;
    pub const INT_EVENT_SET: u32 = 0x080;
    pub const INT_EVENT_CLEAR: u32 = 0x084;
    pub const INT_MASK_SET: u32 = 0x088;
    pub const INT_MASK_CLEAR: u32 = 0x08c;
    pub const FAIRNESS_CONTROL: u32 = 0x0dc;
    pub const LINK_CONTROL_SET: u32 = 0x0e0;
    pub const LINK_CONTROL_CLEAR: u32 = 0x0e4;
    pub const PHY_CONTROL: u32 = 0x0ec;
    pub const AS_REQ_FILTER_HI_SET: u32 = 0x100;
    pub const AS_REQ_FILTER_LO_SET: u32 = 0x108;

    /// Async DMA contexts. Each is a 0x20-byte block: ContextControlSet at
    /// +0x00, ContextControlClear at +0x04, CommandPtr at +0x0c.
    pub const AS_REQ_TR_CONTEXT: u32 = 0x180;
    pub const AS_RSP_TR_CONTEXT: u32 = 0x1a0;
    pub const AS_REQ_RCV_CONTEXT: u32 = 0x1c0;
    pub const AS_RSP_RCV_CONTEXT: u32 = 0x1e0;

    pub const CONTEXT_LEN: u32 = 0x20;
    pub const CONTEXT_CONTROL_SET: u32 = 0x00;
    pub const CONTEXT_CONTROL_CLEAR: u32 = 0x04;
    pub const COMMAND_PTR: u32 = 0x0c;
}

/// `HCControl` bits.
///
/// RetailOS brings the link up with the canonical OHCI sequence: softReset,
/// LPS, postedWriteEnable, linkEnable -- each followed by a read-back.
mod hc_control {
    /// Self-clearing. The firmware polls for it to come back down.
    pub const SOFT_RESET: usize = 16;
}

/// `PhyControl` (0xec).
///
/// The firmware drives it as: write `REG_ADDR` with `RD_REG` set, then spin
/// until `RD_DONE` comes up and take the result out of `RD_DATA`. Linux does
/// the identical handshake -- `ohci1394.c:237` polls `PhyControl & 0x80000000`.
///
/// RetailOS reads PHY register 5 (a single write of 0x0000_8500); diagnostics
/// reads PHY register 8 (built up across two writes into 0x0000_8800).
mod phy_control {
    use std::ops::RangeInclusive;

    /// PHY register to access.
    pub const REG_ADDR: RangeInclusive<usize> = 8..=11;
    /// Kicks off a write. Cleared once the transfer completes.
    pub const WR_REG: usize = 14;
    /// Kicks off a read. Cleared once the transfer completes.
    pub const RD_REG: usize = 15;
    /// Value read back from the PHY.
    pub const RD_DATA: RangeInclusive<usize> = 16..=23;
    /// Address the `RD_DATA` value came from.
    pub const RD_ADDR: RangeInclusive<usize> = 24..=28;
    /// Set once a read has completed.
    pub const RD_DONE: usize = 31;
}

#[derive(Debug, Default)]
struct Context {
    control: u32,
    /// Bits 3..0 are `Z` (the descriptor count), the rest is the 16-byte
    /// aligned address of the descriptor block. Nothing ever fetches these,
    /// since there's no bus to run transfers on.
    command_ptr: u32,
}

#[derive(Debug)]
pub struct Firewire {
    hc_control: u32,
    link_control: u32,
    int_event: u32,
    int_mask: u32,
    phy_control: u32,
    self_id_buffer: u32,

    /// ATRQ, ATRS, ARRQ, ARRS -- in that order.
    contexts: [Context; 4],

    /// Backing store for the parts of the window we haven't identified.
    /// The observed window is 0x200 bytes, addressed 1:1.
    reg: Box<[u32; 0x80]>,
}

impl Firewire {
    pub fn new() -> Firewire {
        Firewire {
            hc_control: 0,
            link_control: 0,
            int_event: 0,
            int_mask: 0,
            phy_control: 0,
            self_id_buffer: 0,
            contexts: Default::default(),
            reg: Box::new([0; 0x80]),
        }
    }

    /// Read a PHY register. All fields set to zero, because we don't
    /// want to emulate an actual FireWire host.
    fn read_phy(&self, _addr: u8) -> u8 {
        0
    }

    /// Map a window offset onto one of the four async DMA contexts, returning
    /// the context index and the offset within its 0x20-byte block.
    fn context_at(offset: u32) -> Option<(usize, u32)> {
        let idx = match offset & !(reg::CONTEXT_LEN - 1) {
            reg::AS_REQ_TR_CONTEXT => 0,
            reg::AS_RSP_TR_CONTEXT => 1,
            reg::AS_REQ_RCV_CONTEXT => 2,
            reg::AS_RSP_RCV_CONTEXT => 3,
            _ => return None,
        };

        Some((idx, offset & (reg::CONTEXT_LEN - 1)))
    }
}

impl Device for Firewire {
    fn kind(&self) -> &'static str {
        "Firewire (OHCI)"
    }

    fn probe(&self, offset: u32) -> Probe {
        if let Some((idx, off)) = Firewire::context_at(offset) {
            let ctx = ["ATRQ", "ATRS", "ARRQ", "ARRS"][idx];
            return Probe::Register(match off {
                reg::CONTEXT_CONTROL_SET => ctx,
                reg::CONTEXT_CONTROL_CLEAR => ctx,
                reg::COMMAND_PTR => ctx,
                _ => "(?) context",
            });
        }

        let reg = match offset {
            reg::AT_RETRIES => "ATRetries",
            reg::HC_CONTROL_SET => "HCControlSet",
            reg::HC_CONTROL_CLEAR => "HCControlClear",
            reg::SELF_ID_BUFFER => "SelfIDBuffer",
            reg::INT_EVENT_SET => "IntEventSet",
            reg::INT_EVENT_CLEAR => "IntEventClear",
            reg::INT_MASK_SET => "IntMaskSet",
            reg::INT_MASK_CLEAR => "IntMaskClear",
            reg::FAIRNESS_CONTROL => "FairnessControl",
            reg::LINK_CONTROL_SET => "LinkControlSet",
            reg::LINK_CONTROL_CLEAR => "LinkControlClear",
            reg::PHY_CONTROL => "PhyControl",
            reg::AS_REQ_FILTER_HI_SET => "AsReqFilterHiSet",
            reg::AS_REQ_FILTER_LO_SET => "AsReqFilterLoSet",
            _ => "(?)",
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

        if let Some((ctx, off)) = Firewire::context_at(offset) {
            let val = match off {
                // The Set and Clear aliases read back the same value.
                reg::CONTEXT_CONTROL_SET | reg::CONTEXT_CONTROL_CLEAR => self.contexts[ctx].control,
                reg::COMMAND_PTR => self.contexts[ctx].command_ptr,
                _ => self.reg[idx],
            };
            return Err(StubRead(Debug, val));
        }

        let val = match offset {
            reg::HC_CONTROL_SET | reg::HC_CONTROL_CLEAR => self.hc_control,
            reg::SELF_ID_BUFFER => self.self_id_buffer,
            reg::INT_EVENT_SET | reg::INT_EVENT_CLEAR => self.int_event,
            reg::INT_MASK_SET | reg::INT_MASK_CLEAR => self.int_mask,
            reg::LINK_CONTROL_SET | reg::LINK_CONTROL_CLEAR => self.link_control,
            reg::PHY_CONTROL => self.phy_control,
            _ => self.reg[idx],
        };

        Err(StubRead(Debug, val))
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        if offset & 0b11 != 0 {
            return Err(Misaligned);
        }

        let idx = (offset / 4) as usize;
        if idx >= self.reg.len() {
            return Err(Unexpected);
        }

        if let Some((ctx, off)) = Firewire::context_at(offset) {
            match off {
                reg::CONTEXT_CONTROL_SET => self.contexts[ctx].control |= val,
                reg::CONTEXT_CONTROL_CLEAR => self.contexts[ctx].control &= !val,
                reg::COMMAND_PTR => self.contexts[ctx].command_ptr = val,
                _ => self.reg[idx] = val,
            }
            return Err(StubWrite(Debug, ()));
        }

        match offset {
            reg::HC_CONTROL_SET => {
                self.hc_control |= val;
                // softReset never stays set: the reset completes instantly, and
                // the firmware polls for the bit to come back down.
                self.hc_control.set_bit(hc_control::SOFT_RESET, false);
            }
            reg::HC_CONTROL_CLEAR => self.hc_control &= !val,

            reg::SELF_ID_BUFFER => self.self_id_buffer = val,

            reg::INT_EVENT_SET => self.int_event |= val,
            reg::INT_EVENT_CLEAR => self.int_event &= !val,
            reg::INT_MASK_SET => self.int_mask |= val,
            reg::INT_MASK_CLEAR => self.int_mask &= !val,

            reg::LINK_CONTROL_SET => self.link_control |= val,
            reg::LINK_CONTROL_CLEAR => self.link_control &= !val,

            reg::PHY_CONTROL => {
                let mut val = val;

                // Transfers complete instantly: there's no bus to arbitrate
                // for, so there's nothing to make the firmware wait on. Leaving
                // RD_DONE clear instead would wedge it -- diagnostics spins on
                // that bit at pc 0x1000bfb0 with no timeout.
                if val.get_bit(phy_control::RD_REG) {
                    let addr = val.get_bits(phy_control::REG_ADDR) as u8;
                    let data = self.read_phy(addr);

                    val.set_bit(phy_control::RD_REG, false)
                        .set_bits(phy_control::RD_ADDR, addr as u32)
                        .set_bits(phy_control::RD_DATA, data as u32)
                        .set_bit(phy_control::RD_DONE, true);
                }

                // Writes to the PHY go nowhere, but the bit still has to clear.
                val.set_bit(phy_control::WR_REG, false);

                self.phy_control = val;
            }

            _ => self.reg[idx] = val,
        }

        Err(StubWrite(Debug, ()))
    }
}

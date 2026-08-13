use crate::devices::prelude::*;
use bit_field::BitField;

/// ARC/Freescale USB-OTG controller, as found on the PP5020 at 0xc5000000.
///
/// Register layout per rockbox's `firmware/target/arm/usb-drv-arc.c`, which
/// drives this exact core (`USB_BASE` is named in its `pp5020.h`).
///
/// This is a stub -- enough for firmware to probe and reset the controller
/// without hanging, but no endpoints, transfers, or attached host are modelled.
#[derive(Debug)]
pub struct Usb {
    reg: Box<[u32; 0x80]>,
}

mod reg {
    /// Run/stop + controller reset.
    pub const USBCMD: u32 = 0x140;
    /// Port status & control.
    pub const PORTSC1: u32 = 0x184;
    /// Endpoint prime.
    pub const ENDPTPRIME: u32 = 0x1b0;
    /// Endpoint flush.
    pub const ENDPTFLUSH: u32 = 0x1b4;

    /// OTG status & control.
    pub const OTGSC: u32 = 0x1a4;
    pub const OTGSC_ID: usize = 8;
    pub const OTGSC_BSV: usize = 11;

    /// `USBCMD` bit 1. Self-clearing: firmware sets it and spins until the
    /// controller takes it back down again.
    pub const USBCMD_CTRL_RESET: usize = 1;
}

impl Usb {
    pub fn new() -> Usb {
        Usb {
            reg: Box::new([0; 0x80]),
        }
    }
}

impl Device for Usb {
    fn kind(&self) -> &'static str {
        "ARC USB-OTG"
    }

    fn probe(&self, offset: u32) -> Probe {
        let name = match offset {
            0x000 => "Id",
            0x004 => "HwGeneral",
            0x008 => "HwHost",
            0x00c => "HwDevice",
            0x010 => "TxBuf",
            0x014 => "RxBuf",
            0x100 => "CapLength",
            0x120 => "DciVersion",
            0x124 => "DccParams",
            0x140 => "UsbCmd",
            0x144 => "UsbSts",
            0x148 => "UsbIntr",
            0x14c => "FrIndex",
            0x154 => "DeviceAddr",
            0x158 => "EndpointListAddr",
            0x160 => "BurstSize",
            0x170 => "Ulpi",
            0x180 => "ConfigFlag",
            0x184 => "PortSc1",
            0x1a4 => "OtgSc",
            0x1a8 => "UsbMode",
            0x1ac => "EndptSetupStat",
            0x1b0 => "EndptPrime",
            0x1b4 => "EndptFlush",
            0x1b8 => "EndptStatus",
            0x1bc => "EndptComplete",
            0x1c0..=0x1fc => "EndptCtrl<X>",
            _ => "?",
        };

        Probe::Register(name)
    }
}

impl Memory for Usb {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            reg::PORTSC1 => Err(StubRead(Debug, 0)), // Port status
            reg::OTGSC => Err(StubRead(Debug, {
                let mut val = self.reg[(offset / 4) as usize];
                val.set_bit(reg::OTGSC_ID, true) // Is a peripheral
                   .set_bit(reg::OTGSC_BSV, true); // Receiving VBUS
                val
            })),
            0x000..=0x1ff => Err(StubRead(Debug, self.reg[(offset / 4) as usize])),
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        let val = match offset {
            reg::USBCMD => {
                let mut val = val;
                val.set_bit(reg::USBCMD_CTRL_RESET, false); // USBCMD_CTRL_RESET bit is self-cleared
                val
            },
            reg::ENDPTPRIME | reg::ENDPTFLUSH => 0, // Self-clearing
            _ => val,
        };

        match offset {
            0x000..=0x1ff => Err(StubWrite(Debug, self.reg[(offset / 4) as usize] = val)),
            _ => Err(Unexpected),
        }
    }
}

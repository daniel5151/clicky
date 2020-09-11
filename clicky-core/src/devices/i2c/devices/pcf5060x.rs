use crate::devices::i2c::prelude::*;

use std::convert::TryFrom;

use chrono::{Datelike, Local, Timelike};
use num_enum::TryFromPrimitive;

/// PCF5060x - Controller for Power Supply and Battery Management + RTC
#[derive(Debug)]
pub struct Pcf5060x {
    last_op_was_write: bool,
    register: Option<u8>,
    inner: Pcf5060xImpl,
}

impl Pcf5060x {
    pub fn new() -> Pcf5060x {
        Pcf5060x {
            last_op_was_write: false,
            register: None,
            inner: Pcf5060xImpl::new(),
        }
    }
}

impl Device for Pcf5060x {
    fn kind(&self) -> &'static str {
        "Pcf5060x"
    }

    fn probe(&self, _offset: u32) -> Probe {
        let reg = match self.register {
            Some(reg) => reg,
            None => return Probe::Register("<no register selected>"),
        };

        // need to subtract 1 due to auto increment behavior
        let reg = match reg - 1 {
            0x00 => "ID",
            0x01 => "OOCS",
            0x02 => "INT1",
            0x03 => "INT2",
            0x04 => "INT3",
            0x05 => "INT1M",
            0x06 => "INT2M",
            0x07 => "INT3M",
            0x08 => "OOCC1",
            0x09 => "OOCC2",
            0x0a => "RTCSC",
            0x0b => "RTCMN",
            0x0c => "RTCHR",
            0x0d => "RTCWD",
            0x0e => "RTCDT",
            0x0f => "RTCMT",
            0x10 => "RTCYR",
            0x11 => "RTCSCA",
            0x12 => "RTCMNA",
            0x13 => "RTCHRA",
            0x14 => "RTCWDA",
            0x15 => "RTCDTA",
            0x16 => "RTCMTA",
            0x17 => "RTCYRA",
            0x18 => "PSSC",
            0x19 => "PWROKM",
            0x1a => "PWROKS",
            0x1b => "DCDC1",
            0x1c => "DCDC2",
            0x1d => "DCDC3",
            0x1e => "DCDC4",
            0x1f => "DCDEC1",
            0x20 => "DCDEC2",
            0x21 => "DCUDC1",
            0x22 => "DCUDC2",
            0x23 => "IOREGC",
            0x24 => "D1REGC1",
            0x25 => "D2REGC1",
            0x26 => "D3REGC1",
            0x27 => "LPREGC1",
            0x28 => "LPREGC2",
            0x29 => "MBCC1",
            0x2a => "MBCC2",
            0x2b => "MBCC3",
            0x2c => "MBCS1",
            0x2d => "BBCC",
            0x2e => "ADCC1",
            0x2f => "ADCC2",
            0x30 => "ADCS1",
            0x31 => "ADCS2",
            0x32 => "ADCS3",
            0x33 => "ACDC1",
            0x34 => "BVMC",
            0x35 => "PWMC1",
            0x36 => "LEDC1",
            0x37 => "LEDC2",
            0x38 => "GPOC1",
            0x39 => "GPOC2",
            0x3a => "GPOC3",
            0x3b => "GPOC4",
            0x3c => "GPOC5",
            _ => "<invalid>",
        };

        Probe::Register(reg)
    }
}

impl I2CDevice for Pcf5060x {
    fn read(&mut self) -> MemResult<u8> {
        self.last_op_was_write = false;

        match self.register {
            None => Err(Fatal("no register specified for read".into())),
            Some(ref mut reg) => {
                let reg_ = *reg;
                *reg += 1;
                let reg = Reg::try_from(reg_).map_err(|_| Fatal("invalid register".into()))?;
                self.inner.read(reg)
            }
        }
    }

    fn write(&mut self, data: u8) -> MemResult<()> {
        if !self.last_op_was_write {
            self.register = None; // reset the register
        }
        self.last_op_was_write = true;

        match self.register {
            None => Ok(self.register = Some(data)),
            Some(ref mut reg) => {
                let reg_ = *reg;
                *reg += 1;
                let reg = Reg::try_from(reg_).map_err(|_| Fatal("invalid register".into()))?;
                self.inner.write(reg, data)
            }
        }
    }

    fn write_done(&mut self) -> MemResult<()> {
        self.last_op_was_write = false;
        Ok(())
    }
}

#[derive(Debug, TryFromPrimitive)]
#[repr(u8)]
enum Reg {
    ID_____ = 0x00,
    OOCS___ = 0x01,
    INT1___ = 0x02,
    INT2___ = 0x03,
    INT3___ = 0x04,
    INT1M__ = 0x05,
    INT2M__ = 0x06,
    INT3M__ = 0x07,
    OOCC1__ = 0x08,
    OOCC2__ = 0x09,
    RTCSC__ = 0x0a,
    RTCMN__ = 0x0b,
    RTCHR__ = 0x0c,
    RTCWD__ = 0x0d,
    RTCDT__ = 0x0e,
    RTCMT__ = 0x0f,
    RTCYR__ = 0x10,
    RTCSCA_ = 0x11,
    RTCMNA_ = 0x12,
    RTCHRA_ = 0x13,
    RTCWDA_ = 0x14,
    RTCDTA_ = 0x15,
    RTCMTA_ = 0x16,
    RTCYRA_ = 0x17,
    PSSC___ = 0x18,
    PWROKM_ = 0x19,
    PWROKS_ = 0x1a,
    DCDC1__ = 0x1b,
    DCDC2__ = 0x1c,
    DCDC3__ = 0x1d,
    DCDC4__ = 0x1e,
    DCDEC1_ = 0x1f,
    DCDEC2_ = 0x20,
    DCUDC1_ = 0x21,
    DCUDC2_ = 0x22,
    IOREGC_ = 0x23,
    D1REGC1 = 0x24,
    D2REGC1 = 0x25,
    D3REGC1 = 0x26,
    LPREGC1 = 0x27,
    LPREGC2 = 0x28,
    MBCC1__ = 0x29,
    MBCC2__ = 0x2a,
    MBCC3__ = 0x2b,
    MBCS1__ = 0x2c,
    BBCC___ = 0x2d,
    ADCC1__ = 0x2e,
    ADCC2__ = 0x2f,
    ADCS1__ = 0x30,
    ADCS2__ = 0x31,
    ADCS3__ = 0x32,
    ACDC1__ = 0x33,
    BVMC___ = 0x34,
    PWMC1__ = 0x35,
    LEDC1__ = 0x36,
    LEDC2__ = 0x37,
    GPOC1__ = 0x38,
    GPOC2__ = 0x39,
    GPOC3__ = 0x3a,
    GPOC4__ = 0x3b,
    GPOC5__ = 0x3c,
}

#[derive(Debug)]
struct Pcf5060xImpl {
    int_mask: [u8; 3],
    oocc1: u8,
    oocc2: u8,
    lpregc1: u8,
    dxregc1: [u8; 3],
    dcdcx: [u8; 4],
    mbcc2: u8,
    rtc_alarm: [u8; 7],
    bvmc: u8,
}

impl Pcf5060xImpl {
    fn new() -> Pcf5060xImpl {
        Pcf5060xImpl {
            int_mask: [0; 3],
            oocc1: 0,
            oocc2: 0,
            lpregc1: 0,
            dxregc1: [0; 3],
            dcdcx: [0; 4],
            mbcc2: 0,
            rtc_alarm: [0; 7],
            bvmc: 0,
        }
    }

    fn get_current_time(&self, reg: Reg) -> MemResult<u8> {
        fn dec2bcd(x: u8) -> u8 {
            ((x / 10) << 4) | (x % 10)
        }

        let now = Local::now();

        use Reg::*;
        let val = match reg {
            RTCSC__ => now.second() as _,
            RTCMN__ => now.minute() as _,
            RTCHR__ => now.hour() as _,
            RTCWD__ => ((now.weekday().num_days_from_monday() + 1) % 8) as _,
            RTCDT__ => now.day() as _,
            RTCMT__ => now.month() as _,
            RTCYR__ => (now.year() % 100) as _,
            _ => unreachable!("invalid reg passed to get_current_time"),
        };

        Ok(dec2bcd(val))
    }

    fn set_current_time(&mut self, reg: Reg) -> MemResult<()> {
        fn _bcd2dec(x: u8) -> u8 {
            (((x >> 4) & 0x0f) * 10) + (x & 0xf)
        }

        let _ = reg;
        // TODO: support setting the RTC
        Err(StubWrite(Info, ()))
    }

    fn read(&mut self, reg: Reg) -> MemResult<u8> {
        use Reg::*;
        match reg {
            ID_____ => Err(StubRead(Info, 0)),
            // On/Off control (OOC)
            OOCC1__ => Err(StubRead(Info, self.oocc1 as u32)),
            OOCC2__ => Err(StubRead(Info, self.oocc2 as u32)),
            // low drop-out linear regulators
            LPREGC1 => Ok(self.lpregc1),
            D1REGC1 => Ok(self.dxregc1[0]),
            D2REGC1 => Ok(self.dxregc1[1]),
            D3REGC1 => Ok(self.dxregc1[2]),
            // DC/DC step down converter (DCD)
            DCDC1__ => Ok(self.dcdcx[0]),
            DCDC2__ => Ok(self.dcdcx[1]),
            DCDC3__ => Ok(self.dcdcx[2]),
            DCDC4__ => Ok(self.dcdcx[3]),
            // Main Battery Charger (MBC)
            // maximum charging time watchdog timer
            MBCC2__ => Err(StubRead(Info, self.mbcc2 as u32)),
            // Interrupt Status registers
            // NOTE: reading from INT registers also clears interrupts
            INT1___ => Err(StubRead(Trace, 0)),
            INT2___ => Err(StubRead(Trace, 0)),
            INT3___ => Err(StubRead(Trace, 0)),
            // Interrupt Mask registers
            INT1M__ => Ok(self.int_mask[0]),
            INT2M__ => Ok(self.int_mask[1]),
            INT3M__ => Ok(self.int_mask[2]),
            // RTC registers
            RTCSC__ | RTCMN__ | RTCHR__ | RTCWD__ | RTCDT__ | RTCMT__ | RTCYR__ => {
                self.get_current_time(reg)
            }
            // RTC Alarm registers
            RTCSCA_ => Ok(self.rtc_alarm[0]),
            RTCMNA_ => Ok(self.rtc_alarm[1]),
            RTCHRA_ => Ok(self.rtc_alarm[2]),
            RTCWDA_ => Ok(self.rtc_alarm[3]),
            RTCDTA_ => Ok(self.rtc_alarm[4]),
            RTCMTA_ => Ok(self.rtc_alarm[5]),
            RTCYRA_ => Ok(self.rtc_alarm[6]),
            // Analog / Digital Converter (ADC)
            // TODO: return better values of charging status?
            ADCC2__ => Err(StubRead(Trace, 0)),
            ADCS1__ => Err(StubRead(Trace, 0)),
            ADCS2__ => Err(StubRead(Trace, 0)),
            ADCS3__ => Err(StubRead(Trace, 0)),
            // Battery Voltage Monitor (BVM)
            BVMC___ => Ok(self.bvmc),
            _ => Err(Unimplemented),
        }
    }

    fn write(&mut self, reg: Reg, data: u8) -> MemResult<()> {
        use Reg::*;
        match reg {
            ID_____ => Err(InvalidAccess),
            // On/Off control (OOC)
            OOCC1__ => Err(StubWrite(Info, self.oocc1 = data)),
            OOCC2__ => Err(StubWrite(Info, self.oocc2 = data)),
            // low drop-out linear regulators
            LPREGC1 => Ok(self.lpregc1 = data),
            D1REGC1 => Ok(self.dxregc1[0] = data),
            D2REGC1 => Ok(self.dxregc1[1] = data),
            D3REGC1 => Ok(self.dxregc1[2] = data),
            // DC/DC step down converter (DCD)
            DCDC1__ => Ok(self.dcdcx[0] = data),
            DCDC2__ => Ok(self.dcdcx[1] = data),
            DCDC3__ => Ok(self.dcdcx[2] = data),
            DCDC4__ => Ok(self.dcdcx[3] = data),
            // Main Battery Charger (MBC)
            // maximum charging time watchdog timer
            MBCC2__ => Err(StubWrite(Info, self.mbcc2 = data)),
            // Interrupt Status registers
            INT1___ | INT2___ | INT3___ => Err(InvalidAccess),
            // Interrupt Mask registers
            INT1M__ => Ok(self.int_mask[0] = data),
            INT2M__ => Ok(self.int_mask[1] = data),
            INT3M__ => Ok(self.int_mask[2] = data),
            // RTC registers
            RTCSC__ | RTCMN__ | RTCHR__ | RTCWD__ | RTCDT__ | RTCMT__ | RTCYR__ => {
                self.set_current_time(reg)
            }
            // RTC Alarm registers
            RTCSCA_ => Ok(self.rtc_alarm[0] = data),
            RTCMNA_ => Ok(self.rtc_alarm[1] = data),
            RTCHRA_ => Ok(self.rtc_alarm[2] = data),
            RTCWDA_ => Ok(self.rtc_alarm[3] = data),
            RTCDTA_ => Ok(self.rtc_alarm[4] = data),
            RTCMTA_ => Ok(self.rtc_alarm[5] = data),
            RTCYRA_ => Ok(self.rtc_alarm[6] = data),
            // Analog / Digital Converter (ADC)
            ADCC2__ => Err(StubWrite(Trace, ())),
            ADCS1__ => Err(InvalidAccess),
            ADCS2__ => Err(InvalidAccess),
            ADCS3__ => Err(InvalidAccess),
            // Battery Voltage Monitor (BVM)
            BVMC___ => Ok(self.bvmc = data),
            _ => Err(Unimplemented),
        }
    }
}

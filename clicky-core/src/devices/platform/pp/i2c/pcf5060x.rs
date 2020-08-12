use crate::devices::prelude::*;

use std::convert::TryFrom;

use chrono::{Datelike, Local, Timelike};
use num_enum::TryFromPrimitive;

use super::I2CDevice;

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
            0x0A => "RTCSC",
            0x0B => "RTCMN",
            0x0C => "RTCHR",
            0x0D => "RTCWD",
            0x0E => "RTCDT",
            0x0F => "RTCMT",
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
            0x1A => "PWROKS",
            0x1B => "DCDC1",
            0x1C => "DCDC2",
            0x1D => "DCDC3",
            0x1E => "DCDC4",
            0x1F => "DCDEC1",
            0x20 => "DCDEC2",
            0x21 => "DCUDC1",
            0x22 => "DCUDC2",
            0x23 => "OREGC",
            0x24 => "D1REGC1",
            0x25 => "D2REGC1",
            0x26 => "D3REGC1",
            0x27 => "LPREGC",
            0x28 => "LPREGC2",
            0x29 => "MBCC1",
            0x2A => "MBCC2",
            0x2B => "MBCC3",
            0x2C => "MBCS1",
            0x2D => "BBCC",
            0x2E => "ADCC1",
            0x2F => "ADCC2",
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
            0x3A => "GPOC3",
            0x3B => "GPOC4",
            0x3C => "GPOCS",
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
    RTCSC__ = 0x0A,
    RTCMN__ = 0x0B,
    RTCHR__ = 0x0C,
    RTCWD__ = 0x0D,
    RTCDT__ = 0x0E,
    RTCMT__ = 0x0F,
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
    PWROKS_ = 0x1A,
    DCDC1__ = 0x1B,
    DCDC2__ = 0x1C,
    DCDC3__ = 0x1D,
    DCDC4__ = 0x1E,
    DCDEC1_ = 0x1F,
    DCDEC2_ = 0x20,
    DCUDC1_ = 0x21,
    DCUDC2_ = 0x22,
    OREGC__ = 0x23,
    D1REGC1 = 0x24,
    D2REGC1 = 0x25,
    D3REGC1 = 0x26,
    LPREGC_ = 0x27,
    LPREGC2 = 0x28,
    MBCC1__ = 0x29,
    MBCC2__ = 0x2A,
    MBCC3__ = 0x2B,
    MBCS1__ = 0x2C,
    BBCC___ = 0x2D,
    ADCC1__ = 0x2E,
    ADCC2__ = 0x2F,
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
    GPOC3__ = 0x3A,
    GPOC4__ = 0x3B,
    GPOCS__ = 0x3C,
}

#[derive(Debug)]
struct Pcf5060xImpl {}

impl Pcf5060xImpl {
    fn new() -> Pcf5060xImpl {
        Pcf5060xImpl {}
    }

    fn get_current_time(&self, reg: Reg) -> MemResult<u8> {
        fn dec2bcd(x: u8) -> u8 {
            ((x / 10) << 4) | (x % 10)
        }

        let now = Local::now();

        use Reg::*;
        let val = match reg {
            // RTC registers
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
            // RTC registers
            RTCSC__ | RTCMN__ | RTCHR__ | RTCWD__ | RTCDT__ | RTCMT__ | RTCYR__ => {
                self.get_current_time(reg)
            }
            _ => Err(StubRead(Error, 0)),
        }
    }

    fn write(&mut self, reg: Reg, _data: u8) -> MemResult<()> {
        use Reg::*;
        match reg {
            // RTC registers
            RTCSC__ | RTCMN__ | RTCHR__ | RTCWD__ | RTCDT__ | RTCMT__ | RTCYR__ => {
                self.set_current_time(reg)
            }
            _ => Err(StubWrite(Error, ())),
        }
    }
}

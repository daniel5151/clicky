use crate::devices::i2c::prelude::*;

use std::convert::TryFrom;
use std::time::Duration;

use chrono::{Datelike, Local, Timelike};
use num_enum::TryFromPrimitive;
use relativity::Instant;

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
        self.inner.end_read();
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

#[derive(Clone, Copy, Debug, TryFromPrimitive)]
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

#[derive(Clone, Copy, Debug, PartialEq)]
struct RtcTime {
    second: u8,
    minute: u8,
    hour: u8,
    weekday: u8,
    day: u8,
    month: u8,
    year: u8,
}

impl RtcTime {
    fn now() -> RtcTime {
        let now = Local::now();
        RtcTime {
            second: now.second() as _,
            minute: now.minute() as _,
            hour: now.hour() as _,
            weekday: now.weekday().num_days_from_sunday() as u8,
            day: now.day() as _,
            month: now.month() as _,
            year: (now.year() % 100) as _,
        }
    }

    fn advance(&mut self, seconds: u64) {
        let seconds =
            self.second as u64 + self.minute as u64 * 60 + self.hour as u64 * 60 * 60 + seconds;
        let days = seconds / (24 * 60 * 60);
        let seconds = seconds % (24 * 60 * 60);

        self.hour = (seconds / (60 * 60)) as _;
        self.minute = ((seconds / 60) % 60) as _;
        self.second = (seconds % 60) as _;

        for _ in 0..days {
            self.advance_day();
        }
    }

    fn advance_day(&mut self) {
        self.weekday = (self.weekday + 1) % 7;

        if self.day < days_in_month(self.month, self.year) {
            self.day += 1;
            return;
        }

        self.day = 1;
        if self.month < 12 {
            self.month += 1;
        } else {
            self.month = 1;
            self.year = self.year.wrapping_add(1) % 100;
        }
    }
}

fn days_in_month(month: u8, year: u8) -> u8 {
    match month {
        2 if year % 4 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
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
    gp0c1: u8,
    adcc1: u8,
    adcc2: u8,
    acdc1: u8,
    rtc: RtcTime,
    rtc_last_tick: Instant,
    rtc_write_awaiting_tick: bool,
    rtc_read_latch: Option<RtcTime>,
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
            gp0c1: 0x04,
            adcc1: 0,
            adcc2: 0,
            acdc1: 0,
            rtc: RtcTime::now(),
            rtc_last_tick: Instant::now(),
            rtc_write_awaiting_tick: false,
            rtc_read_latch: None,
        }
    }

    fn end_read(&mut self) {
        self.rtc_read_latch = None;
    }

    fn sync_rtc(&mut self) {
        let now = Instant::now();
        let elapsed_ticks = (now - self.rtc_last_tick).as_secs();
        if elapsed_ticks == 0 {
            return;
        }

        // A written value is latched by the first 1 Hz tick, not advanced by it.
        let elapsed = if self.rtc_write_awaiting_tick {
            self.rtc_write_awaiting_tick = false;
            elapsed_ticks - 1
        } else {
            elapsed_ticks
        };
        self.rtc.advance(elapsed);
        self.rtc_last_tick += Duration::from_secs(elapsed_ticks);
    }

    fn get_current_time(&mut self, reg: Reg) -> MemResult<u8> {
        fn dec2bcd(x: u8) -> u8 {
            ((x / 10) << 4) | (x % 10)
        }

        if self.rtc_read_latch.is_none() {
            self.sync_rtc();
            // Keep auto-increment reads from straddling a one-second boundary.
            self.rtc_read_latch = Some(self.rtc);
        }
        let rtc = self.rtc_read_latch.unwrap();

        use Reg::*;
        let val = match reg {
            RTCSC__ => rtc.second,
            RTCMN__ => rtc.minute,
            RTCHR__ => rtc.hour,
            RTCWD__ => rtc.weekday,
            RTCDT__ => rtc.day,
            RTCMT__ => rtc.month,
            RTCYR__ => rtc.year,
            _ => unreachable!("invalid reg passed to get_current_time"),
        };

        Ok(dec2bcd(val))
    }

    fn set_current_time(&mut self, reg: Reg, data: u8) -> MemResult<()> {
        fn bcd2dec(data: u8, mask: u8, min: u8, max: u8) -> Option<u8> {
            let data = data & mask;
            let high = data >> 4;
            let low = data & 0x0f;
            if high > 9 || low > 9 {
                return None;
            }

            let value = high * 10 + low;
            if (min..=max).contains(&value) {
                Some(value)
            } else {
                None
            }
        }

        self.sync_rtc();
        self.rtc_read_latch = None;

        use Reg::*;
        let (value, field) = match reg {
            RTCSC__ => (bcd2dec(data, 0x7f, 0, 59), &mut self.rtc.second),
            RTCMN__ => (bcd2dec(data, 0x7f, 0, 59), &mut self.rtc.minute),
            RTCHR__ => (bcd2dec(data, 0x3f, 0, 23), &mut self.rtc.hour),
            RTCWD__ => (bcd2dec(data, 0x07, 0, 6), &mut self.rtc.weekday),
            RTCDT__ => (bcd2dec(data, 0x3f, 1, 31), &mut self.rtc.day),
            RTCMT__ => (bcd2dec(data, 0x1f, 1, 12), &mut self.rtc.month),
            RTCYR__ => (bcd2dec(data, 0xff, 0, 99), &mut self.rtc.year),
            _ => unreachable!("invalid reg passed to set_current_time"),
        };

        *field = value.ok_or(InvalidAccess)?;
        self.rtc_write_awaiting_tick = true;
        Ok(())
    }

    fn get_adc_readout(&mut self, reg: Reg) -> MemResult<u8> {
        const ADCRDY: u32 = 0x80;

        let mux_sel = self.adcc2.get_bits(1..=4);
        let readout = match mux_sel {
            0 => 621, // BATVOLT, resistive divider
            1 => 232, // BATVOLT, substractor
            2 => 621, // ADCIN1, resistive divider
            3 => 621, // ADCIN1, substractor
            4 => 385, // BATTEMP, radiometric
            _ => return Err(Unimplemented),
        };

        use Reg::*;
        match reg {
            ADCS1__ => Err(StubRead(Info, (readout >> 2) & 0xff)),
            ADCS2__ => Err(StubRead(Info, ADCRDY | (readout & 0x3))),
            ADCS3__ => Err(StubRead(Info, 0)), // Warning: ADCDAT2 is NOT emulated!
            _ => Err(Unimplemented),
        }
    }

    fn read(&mut self, reg: Reg) -> MemResult<u8> {
        use Reg::*;
        if !matches!(
            reg,
            RTCSC__ | RTCMN__ | RTCHR__ | RTCWD__ | RTCDT__ | RTCMT__ | RTCYR__
        ) {
            self.end_read();
        }

        match reg {
            ID_____ => Ok(74),
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
            ADCC1__ => Ok(self.adcc1),
            ADCC2__ => Ok(self.adcc2 & 0b1111_1110), // Don't store ADCSTART bit
            ADCS1__ | ADCS2__ | ADCS3__ => self.get_adc_readout(reg),
            ACDC1__ => Ok(self.acdc1),
            // Battery Voltage Monitor (BVM)
            BVMC___ => Ok(self.bvmc),
            GPOC1__ => Ok(self.gp0c1),
            _ => Err(Unimplemented),
        }
    }

    fn write(&mut self, reg: Reg, data: u8) -> MemResult<()> {
        self.end_read();

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
                self.set_current_time(reg, data)
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
            ADCC1__ => Ok(self.adcc1 = data & 0b0111_1111), // Bit 7 is read-only (TSCINT)
            ADCC2__ => Ok(self.adcc2 = data),
            ADCS1__ => Err(InvalidAccess),
            ADCS2__ => Err(InvalidAccess),
            ADCS3__ => Err(InvalidAccess),
            ACDC1__ => Ok(self.acdc1 = data & 0b1001_1110), // Writable bitmask
            // Battery Voltage Monitor (BVM)
            BVMC___ => Ok(self.bvmc = data),
            GPOC1__ => Ok(self.gp0c1 = data),
            _ => Err(Unimplemented),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rtc_time(
        second: u8,
        minute: u8,
        hour: u8,
        weekday: u8,
        day: u8,
        month: u8,
        year: u8,
    ) -> RtcTime {
        RtcTime {
            second,
            minute,
            hour,
            weekday,
            day,
            month,
            year,
        }
    }

    fn inner_with_rtc(rtc: RtcTime) -> Pcf5060xImpl {
        let mut inner = Pcf5060xImpl::new();
        inner.rtc = rtc;
        inner.rtc_last_tick = Instant::now();
        inner
    }

    fn read_i2c_register(device: &mut Pcf5060x, reg: Reg) -> u8 {
        I2CDevice::write(device, reg as u8).unwrap();
        I2CDevice::write_done(device).unwrap();
        I2CDevice::read(device).unwrap()
    }

    #[test]
    fn rtc_hour_and_minute_writes_round_trip_through_i2c() {
        let mut device = Pcf5060x::new();
        device.inner = inner_with_rtc(rtc_time(56, 34, 12, 3, 14, 8, 26));

        I2CDevice::write(&mut device, Reg::RTCMN__ as u8).unwrap();
        I2CDevice::write(&mut device, 0x45).unwrap();
        I2CDevice::write_done(&mut device).unwrap();

        I2CDevice::write(&mut device, Reg::RTCHR__ as u8).unwrap();
        I2CDevice::write(&mut device, 0x21).unwrap();
        I2CDevice::write_done(&mut device).unwrap();

        assert_eq!(read_i2c_register(&mut device, Reg::RTCSC__), 0x56);
        assert_eq!(read_i2c_register(&mut device, Reg::RTCMN__), 0x45);
        assert_eq!(read_i2c_register(&mut device, Reg::RTCHR__), 0x21);
        assert_eq!(read_i2c_register(&mut device, Reg::RTCWD__), 0x03);
        assert_eq!(read_i2c_register(&mut device, Reg::RTCDT__), 0x14);
        assert_eq!(read_i2c_register(&mut device, Reg::RTCMT__), 0x08);
        assert_eq!(read_i2c_register(&mut device, Reg::RTCYR__), 0x26);
    }

    #[test]
    fn rtc_advances_across_time_and_calendar_boundaries() {
        let mut rtc = rtc_time(59, 34, 12, 3, 14, 8, 26);
        rtc.advance(1);
        assert_eq!(rtc, rtc_time(0, 35, 12, 3, 14, 8, 26));

        let mut rtc = rtc_time(59, 59, 23, 3, 28, 2, 24);
        rtc.advance(1);
        assert_eq!(rtc, rtc_time(0, 0, 0, 4, 29, 2, 24));

        let mut rtc = rtc_time(59, 59, 23, 6, 31, 12, 99);
        rtc.advance(1);
        assert_eq!(rtc, rtc_time(0, 0, 0, 0, 1, 1, 0));
    }

    #[test]
    fn all_rtc_registers_round_trip_valid_masked_bcd_values() {
        let mut inner = inner_with_rtc(rtc_time(0, 0, 0, 1, 1, 1, 0));
        let values = [
            (Reg::RTCSC__, 0xd8, 0x58),
            (Reg::RTCMN__, 0xd9, 0x59),
            (Reg::RTCHR__, 0xe3, 0x23),
            (Reg::RTCWD__, 0xf0, 0x00),
            (Reg::RTCDT__, 0xf1, 0x31),
            (Reg::RTCMT__, 0xf2, 0x12),
            (Reg::RTCYR__, 0x99, 0x99),
        ];

        for (reg, written, expected) in values {
            inner.write(reg, written).unwrap();
            assert_eq!(inner.read(reg).unwrap(), expected);
        }
    }

    #[test]
    fn invalid_rtc_writes_leave_the_clock_unchanged() {
        let initial = rtc_time(56, 34, 12, 3, 14, 8, 26);
        let invalid_values = [
            (Reg::RTCSC__, 0x5a),
            (Reg::RTCSC__, 0x60),
            (Reg::RTCMN__, 0x4a),
            (Reg::RTCMN__, 0x60),
            (Reg::RTCHR__, 0x1a),
            (Reg::RTCHR__, 0x24),
            (Reg::RTCWD__, 0x07),
            (Reg::RTCDT__, 0x2a),
            (Reg::RTCDT__, 0x32),
            (Reg::RTCMT__, 0x1a),
            (Reg::RTCMT__, 0x13),
            (Reg::RTCYR__, 0xa0),
        ];

        for (reg, value) in invalid_values {
            let mut inner = inner_with_rtc(initial);
            assert!(matches!(inner.write(reg, value), Err(InvalidAccess)));
            assert_eq!(inner.rtc, initial);
            assert!(!inner.rtc_write_awaiting_tick);
        }
    }

    #[test]
    fn rtc_write_takes_effect_on_the_next_tick() {
        let mut inner = inner_with_rtc(rtc_time(10, 20, 12, 3, 14, 8, 26));
        inner.write(Reg::RTCMN__, 0x34).unwrap();

        inner.rtc_last_tick = Instant::now() - Duration::from_secs(1);
        inner.sync_rtc();
        assert_eq!(inner.rtc, rtc_time(10, 34, 12, 3, 14, 8, 26));

        inner.rtc_last_tick = Instant::now() - Duration::from_secs(1);
        inner.sync_rtc();
        assert_eq!(inner.rtc, rtc_time(11, 34, 12, 3, 14, 8, 26));
    }

    #[test]
    fn consecutive_i2c_reads_use_a_coherent_rtc_snapshot() {
        let mut device = Pcf5060x::new();
        device.inner = inner_with_rtc(rtc_time(59, 59, 23, 6, 31, 12, 99));

        I2CDevice::write(&mut device, Reg::RTCSC__ as u8).unwrap();
        I2CDevice::write_done(&mut device).unwrap();

        let second = I2CDevice::read(&mut device).unwrap();
        device.inner.rtc.advance(1);
        let remaining = (0..6)
            .map(|_| I2CDevice::read(&mut device).unwrap())
            .collect::<Vec<_>>();

        assert_eq!(second, 0x59);
        assert_eq!(remaining, [0x59, 0x23, 0x06, 0x31, 0x12, 0x99]);
    }
}

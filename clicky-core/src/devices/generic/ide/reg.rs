#![allow(non_snake_case, unused)]

/// status register bits
pub mod STATUS {
    /// Busy
    pub const BSY: usize = 7;
    /// Device Ready
    pub const DRDY: usize = 6;
    /// Device Fault
    pub const DF: usize = 5;
    /// Disk Seek Complete
    pub const DSC: usize = 4;
    /// Data Request
    pub const DRQ: usize = 3;
    /// Corrected Data
    pub const CORR: usize = 2;
    /// Index (vendor specific)
    pub const IDX: usize = 1;
    /// Error
    pub const ERR: usize = 0;
}

/// Error register bits
pub mod ERROR {
    /// Unrecoverable Data Error
    pub const UNC: usize = 6;
    /// Media Changed
    pub const MC: usize = 5;
    /// ID Not found
    pub const IDNF: usize = 4;
    /// Media Change Requested
    pub const MCR: usize = 3;
    /// Aborted Command
    pub const ABRT: usize = 2;
    /// Track 0 Not Found (during a RECALIBRATE command)
    pub const TKNONF: usize = 1;
    /// Address Mark Not Found
    pub const AMNF: usize = 0;
}

/// Device/Head register bits
pub mod DEVHEAD {
    type Range = std::ops::RangeInclusive<usize>;
    /// LBA addressing
    pub const L: usize = 6;
    /// Device Index
    pub const DEV: usize = 4;
    /// Bits 24..=27 of the LBA address
    pub const HS: Range = 0..=3;
}

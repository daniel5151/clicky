/// Structure returned by the ATA IDENTITY command.
///
/// Copied from include/linux/hdreg.h.
#[derive(Copy, Clone)]
#[repr(C, packed)]
#[allow(non_camel_case_types, non_snake_case)]
pub struct hd_driveid {
    pub config: u16,         /* lots of obsolete bit flags */
    pub cyls: u16,           /* Obsolete, "physical" cyls */
    pub reserved2: u16,      /* reserved (word 2) */
    pub heads: u16,          /* Obsolete, "physical" heads */
    pub track_bytes: u16,    /* unformatted bytes per track */
    pub sector_bytes: u16,   /* unformatted bytes per sector */
    pub sectors: u16,        /* Obsolete, "physical" sectors per track */
    pub vendor0: u16,        /* vendor unique */
    pub vendor1: u16,        /* vendor unique */
    pub vendor2: u16,        /* Retired vendor unique */
    pub serial_no: [u8; 20], /* 0 = not_specified */
    pub buf_type: u16,       /* Retired */
    pub buf_size: u16,       /* Retired, 512 byte increments
                              * 0 = not_specified
                              */
    pub ecc_bytes: u16,   /* for r/w long cmds, 0 = not_specified */
    pub fw_rev: [u8; 8],  /* 0 = not_specified */
    pub model: [u8; 40],  /* 0 = not_specified */
    pub max_multsect: u8, /* 0=not_implemented */
    pub vendor3: u8,      /* vendor unique */
    pub dword_io: u16,    /* 0=not_implemented, 1=implemented */
    pub vendor4: u8,      /* vendor unique */
    pub capability: u8,   /* (upper byte of word 49)
                           *  3:        IORDYsup
                           *  2:        IORDYsw
                           *  1:        LBA
                           *  0:        DMA
                           */
    pub reserved50: u16, /* reserved (word 50) */
    pub vendor5: u8,     /* Obsolete, vendor unique */
    pub tPIO: u8,        /* Obsolete, 0=slow, 1=medium, 2=fast */
    pub vendor6: u8,     /* Obsolete, vendor unique */
    pub tDMA: u8,        /* Obsolete, 0=slow, 1=medium, 2=fast */
    pub field_valid: u16, /* (word 53)
                          *  2:        ultra_ok        word  88
                          *  1:        eide_ok                words 64-70
                          *  0:        cur_ok                words 54-58
                          */
    pub cur_cyls: u16,       /* Obsolete, logical cylinders */
    pub cur_heads: u16,      /* Obsolete, l heads */
    pub cur_sectors: u16,    /* Obsolete, l sectors per track */
    pub cur_capacity0: u16,  /* Obsolete, l total sectors on drive */
    pub cur_capacity1: u16,  /* Obsolete, (2 words, misaligned int) */
    pub multsect: u8,        /* current multiple sector count */
    pub multsect_valid: u8,  /* when (bit0==1) multsect is ok */
    pub lba_capacity: u32,   /* Obsolete, total number of sectors */
    pub dma_1word: u16,      /* Obsolete, single-word dma info */
    pub dma_mword: u16,      /* multiple-word dma info */
    pub eide_pio_modes: u16, /* bits 0:mode3 1:mode4 */
    pub eide_dma_min: u16,   /* min mword dma cycle time (ns) */
    pub eide_dma_time: u16,  /* recommended mword dma cycle time (ns) */
    pub eide_pio: u16,       /* min cycle time (ns), no IORDY */
    pub eide_pio_iordy: u16, /* min cycle time (ns), with IORDY */
    pub words69_70: [u16; 2], /* reserved words 69-70
                              * future command overlap and queuing
                              */
    pub words71_74: [u16; 4], /* reserved words 71-74
                               * for IDENTIFY PACKET DEVICE command
                               */
    pub queue_depth: u16,     /* (word 75)
                               * 15:5        reserved
                               *  4:0        Maximum queue depth -1
                               */
    pub words76_79: [u16; 4], /* reserved words 76-79 */
    pub major_rev_num: u16,   /* (word 80) */
    pub minor_rev_num: u16,   /* (word 81) */
    pub command_set_1: u16,   /* (word 82) supported
                               * 15:        Obsolete
                               * 14:        NOP command
                               * 13:        READ_BUFFER
                               * 12:        WRITE_BUFFER
                               * 11:        Obsolete
                               * 10:        Host Protected Area
                               *  9:        DEVICE Reset
                               *  8:        SERVICE Interrupt
                               *  7:        Release Interrupt
                               *  6:        look-ahead
                               *  5:        write cache
                               *  4:        PACKET Command
                               *  3:        Power Management Feature Set
                               *  2:        Removable Feature Set
                               *  1:        Security Feature Set
                               *  0:        SMART Feature Set
                               */
    pub command_set_2: u16, /* (word 83)
                             * 15:        Shall be ZERO
                             * 14:        Shall be ONE
                             * 13:        FLUSH CACHE EXT
                             * 12:        FLUSH CACHE
                             * 11:        Device Configuration Overlay
                             * 10:        48-bit Address Feature Set
                             *  9:        Automatic Acoustic Management
                             *  8:        SET MAX security
                             *  7:        reserved 1407DT PARTIES
                             *  6:        SetF sub-command Power-Up
                             *  5:        Power-Up in Standby Feature Set
                             *  4:        Removable Media Notification
                             *  3:        APM Feature Set
                             *  2:        CFA Feature Set
                             *  1:        READ/WRITE DMA QUEUED
                             *  0:        Download MicroCode
                             */
    pub cfsse: u16, /* (word 84)
                     * cmd set-feature supported extensions
                     * 15:        Shall be ZERO
                     * 14:        Shall be ONE
                     * 13:6        reserved
                     *  5:        General Purpose Logging
                     *  4:        Streaming Feature Set
                     *  3:        Media Card Pass Through
                     *  2:        Media Serial Number Valid
                     *  1:        SMART selt-test supported
                     *  0:        SMART error logging
                     */
    pub cfs_enable_1: u16, /* (word 85)
                            * command set-feature enabled
                            * 15:        Obsolete
                            * 14:        NOP command
                            * 13:        READ_BUFFER
                            * 12:        WRITE_BUFFER
                            * 11:        Obsolete
                            * 10:        Host Protected Area
                            *  9:        DEVICE Reset
                            *  8:        SERVICE Interrupt
                            *  7:        Release Interrupt
                            *  6:        look-ahead
                            *  5:        write cache
                            *  4:        PACKET Command
                            *  3:        Power Management Feature Set
                            *  2:        Removable Feature Set
                            *  1:        Security Feature Set
                            *  0:        SMART Feature Set
                            */
    pub cfs_enable_2: u16, /* (word 86)
                            * command set-feature enabled
                            * 15:        Shall be ZERO
                            * 14:        Shall be ONE
                            * 13:        FLUSH CACHE EXT
                            * 12:        FLUSH CACHE
                            * 11:        Device Configuration Overlay
                            * 10:        48-bit Address Feature Set
                            *  9:        Automatic Acoustic Management
                            *  8:        SET MAX security
                            *  7:        reserved 1407DT PARTIES
                            *  6:        SetF sub-command Power-Up
                            *  5:        Power-Up in Standby Feature Set
                            *  4:        Removable Media Notification
                            *  3:        APM Feature Set
                            *  2:        CFA Feature Set
                            *  1:        READ/WRITE DMA QUEUED
                            *  0:        Download MicroCode
                            */
    pub csf_default: u16,  /* (word 87)
                            * command set-feature default
                            * 15:        Shall be ZERO
                            * 14:        Shall be ONE
                            * 13:6        reserved
                            *  5:        General Purpose Logging enabled
                            *  4:        Valid CONFIGURE STREAM executed
                            *  3:        Media Card Pass Through enabled
                            *  2:        Media Serial Number Valid
                            *  1:        SMART selt-test supported
                            *  0:        SMART error logging
                            */
    pub dma_ultra: u16,    /* (word 88) */
    pub trseuc: u16,       /* time required for security erase */
    pub trsEuc: u16,       /* time required for enhanced erase */
    pub CurAPMvalues: u16, /* current APM values */
    pub mprc: u16,         /* master password revision code */
    pub hw_config: u16,    /* hardware config (word 93)
                            * 15:        Shall be ZERO
                            * 14:        Shall be ONE
                            * 13:
                            * 12:
                            * 11:
                            * 10:
                            *  9:
                            *  8:
                            *  7:
                            *  6:
                            *  5:
                            *  4:
                            *  3:
                            *  2:
                            *  1:
                            *  0:        Shall be ONE
                            */
    pub acoustic: u16,           /* (word 94)
                                  * 15:8        Vendor's recommended value
                                  *  7:0        current value
                                  */
    pub msrqs: u16,              /* min stream request size */
    pub sxfert: u16,             /* stream transfer time */
    pub sal: u16,                /* stream access latency */
    pub spg: u32,                /* stream performance granularity */
    pub lba_capacity_2: u64,     /* 48-bit total number of sectors */
    pub words104_125: [u16; 22], /* reserved words 104-125 */
    pub last_lun: u16,           /* (word 126) */
    pub word127: u16,            /* (word 127) Feature Set
                                  * Removable Media Notification
                                  * 15:2        reserved
                                  *  1:0        00 = not supported
                                  *        01 = supported
                                  *        10 = reserved
                                  *        11 = reserved
                                  */
    pub dlf: u16, /* (word 128)
                   * device lock function
                   * 15:9        reserved
                   *  8        security level 1:max 0:high
                   *  7:6        reserved
                   *  5        enhanced erase
                   *  4        expire
                   *  3        frozen
                   *  2        locked
                   *  1        en/disabled
                   *  0        capability
                   */
    pub csfo: u16,               /*  (word 129)
                                  * current set features options
                                  * 15:4        reserved
                                  *  3:        auto reassign
                                  *  2:        reverting
                                  *  1:        read-look-ahead
                                  *  0:        write cache
                                  */
    pub words130_155: [u16; 26], /* reserved vendor words 130-155 */
    pub word156: u16,            /* reserved vendor word 156 */
    pub words157_159: [u16; 3],  /* reserved vendor words 157-159 */
    pub cfa_power: u16,          /* (word 160) CFA Power Mode
                                  * 15 word 160 supported
                                  * 14 reserved
                                  * 13
                                  * 12
                                  * 11:0
                                  */
    pub words161_175: [u16; 15], /* Reserved for CFA */
    pub words176_205: [u16; 30], /* Current Media Serial Number */
    pub words206_254: [u16; 49], /* reserved words 206-254 */
    pub integrity_word: u16,     /* (word 255)
                                  * 15:8 Checksum
                                  *  7:0 Signature
                                  */
}

// Must be exactly 512 bytes to conform to ATA spec
const_assert_eq!(std::mem::size_of::<hd_driveid>(), 512);

use bytemuck::{Pod, Zeroable};

// Safety:
// - All fields have type `uX` and/or are arrays of `uX` types.
// - #[repr(C, packed)] ensures that there is no padding (and therefore, no
//   invalid bit patterns)
unsafe impl Zeroable for hd_driveid {}
unsafe impl Pod for hd_driveid {}

impl Default for hd_driveid {
    fn default() -> Self {
        Self::zeroed()
    }
}

impl std::fmt::Debug for hd_driveid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "hd_driveid: {{ ... }}")
    }
}

/// IDE Drive Metadata.
///
/// `serial`, `fw_version`, and `model` should be ASCII.
pub struct IdeDriveMeta<'a> {
    /// i.e: (size in bytes) / 512
    pub total_sectors: u64,

    pub cylinders: u16,
    pub heads: u16,
    pub sectors: u16,
    pub serial: &'a [u8],
    pub fw_version: &'a [u8],
    pub model: &'a [u8],
}

impl IdeDriveMeta<'_> {
    /// Populate a `struct hd_driveid` using the provided metadata.
    // NOTE: this method (currently) implements up-to the ATA-2 spec.
    // TODO: add multi-sector support...
    pub fn to_hd_driveid(&self) -> hd_driveid {
        // Some values were cargo-culted from QEMU's source
        // (/hw/ide/core.c:ide_identify).

        let capacity = self.cylinders as u32 * self.heads as u32 * self.sectors as u32;

        let mut id = hd_driveid {
            config: 0x0040, // not removable controller and/or device
            cyls: self.cylinders,
            heads: self.heads,
            track_bytes: self.sectors * 512,
            sector_bytes: 512,
            sectors: self.sectors,
            // serial_no: self.serial, // no ergonomic way to init [u8; N] from &[u8]
            // fw_rev: self.fw_version,
            // model: self.model,
            capability: 0b0111, // DMA and LBA supported, IORDY supported
            field_valid: 0b11,  // words 54-58,64-70 are valid
            cur_cyls: self.cylinders,
            cur_heads: self.heads,
            cur_sectors: self.sectors,
            cur_capacity0: capacity as u16,
            cur_capacity1: (capacity >> 16) as u16,
            lba_capacity: self.total_sectors as u32,

            // (QEMU)
            ecc_bytes: 4,
            tPIO: 2,              // fast
            tDMA: 2,              // fast
            dma_1word: 0x07,      // single word dma0-2 supported
            dma_mword: 0x07,      // mdma0-2 supported
            eide_pio_modes: 0x07, // pio3-4 supported
            eide_dma_min: 120,
            eide_dma_time: 120,
            eide_pio: 120,
            eide_pio_iordy: 120,

            ..hd_driveid::default()
        };

        // writes text into dst buffer, padding with spaces if it's too short. also
        // handles the wonky endianess conversion stuff for strings
        let pad_ascii = |dst: &mut [u8], src: &[u8]| {
            for (i, b) in dst.iter_mut().enumerate() {
                *b = *src.get(i ^ 1).unwrap_or(&b' ');
            }
        };

        pad_ascii(&mut id.serial_no, self.serial);
        pad_ascii(&mut id.fw_rev, self.fw_version);
        pad_ascii(&mut id.model, self.model);

        id
    }
}

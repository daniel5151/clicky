/// Revision of the data structure itself, not the drive.
const REVISION: u16 = 0x0010;

#[derive(Debug)]
pub enum Error {
    TooManyAttributes,
    SimilarAttribute,
}

// Sourced from https://media.kingston.com/kingston/pdf/SMART-attribute.pdf
// and https://ntfs.com/disk-monitor-smart-attributes.htm
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum AttributeId {
    ReadErrorRate = 0x01,
    ThroughputPerformance = 0x02,
    SpinUpTime = 0x03,
    StartStopCount = 0x04,
    ReallocatedSectorCount = 0x05,
    ReadChannelMargin = 0x06,
    SeekErrorRate = 0x07,
    SeekTimePerformance = 0x08,
    PowerOnHours = 0x09,
    SpinRetryCount = 0x0A,
    RecalibrationRetries = 0x0B,
    PowerCycleCount = 0x0C,
    SoftReadErrorRate = 0x0D,
    UnexpectedPowerOffCount = 0xC0,
    Temperature = 0xC2,
    ReallocationEventCount = 0xC4,
}

#[derive(Debug, Copy, Clone)]
pub struct Attribute {
    pub id: u8,
    pub flags: u16,
    pub current: u8,
    pub worst: u8,
    pub raw: u64, // Only 48 bits are available
    pub threshold: u8,
}

impl Attribute {
    pub fn serialize_data(&self) -> [u8; 12] {
        let mut buf: [u8; 12] = [0; 12];

        buf[0] = self.id;
        buf[1..3].copy_from_slice(&self.flags.to_le_bytes());
        buf[3] = self.current;
        buf[4] = self.worst;
        buf[5..11].copy_from_slice(&self.raw.to_le_bytes()[..6]);

        buf
    }

    pub fn serialize_threshold(&self) -> [u8; 12] {
        let mut buf: [u8; 12] = [0; 12];

        buf[0] = self.id;
        buf[1] = self.threshold;

        buf
    }
}

pub struct Smart {
    attributes: Vec<Attribute>,
}

impl Smart {
    pub fn new() -> Smart {
        Smart {
            attributes: Vec::new(),
        }
    }

    pub fn add_attribute(&mut self, attribute: Attribute) -> Result<(), Error> {
        use Error::*;
        if self.attributes.len() >= 12 {
            return Err(TooManyAttributes);
        }

        if self.attributes.iter().any(|x| x.id == attribute.id) {
            return Err(SimilarAttribute);
        }

        self.attributes.push(attribute);
        Ok(())
    }

    fn checksum(data: &[u8; 512]) -> u8 {
        let sum = data[..511].iter().fold(0u8, |acc, b| acc.wrapping_add(*b));
        (0u8).wrapping_sub(sum)
    }

    pub fn serialize_data(&self) -> [u8; 512] {
        let mut buf: [u8; 512] = [0; 512];

        buf[0..2].copy_from_slice(&REVISION.to_le_bytes());

        for (i, &attribute) in self.attributes.iter().enumerate() {
            let offset = 2 + i * 12;
            buf[offset..offset + 12].copy_from_slice(&attribute.serialize_data());
        }

        // off-line collection capability: execute immediate + abort + self-test
        buf[367] = 0x1b;
        // SMART capability: saves attributes before power down, enabled by command
        buf[368..370].copy_from_slice(&0x0003u16.to_le_bytes());
        // error logging supported
        buf[370] = 0x01;
        // recommended self-test polling times, in minutes
        buf[372] = 2;
        buf[373] = 20;

        buf[511] = Smart::checksum(&buf);

        buf
    }

    pub fn serialize_threshold(&self) -> [u8; 512] {
        let mut buf: [u8; 512] = [0; 512];

        buf[0..2].copy_from_slice(&REVISION.to_le_bytes());

        for (i, &attribute) in self.attributes.iter().enumerate() {
            let offset = 2 + i * 12;
            buf[offset..offset+12].copy_from_slice(&attribute.serialize_threshold());
        }

        buf[511] = Smart::checksum(&buf);

        buf
    }
}

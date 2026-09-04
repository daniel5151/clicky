#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PMPModel {
	/// iPod (1st Generation)
    Ipod1g,

    /// iPod (2nd Generation)
    Ipod2g,

    /// iPod (3rd Generation)
    Ipod3g,

    /// iPod (4th Generation)
    Ipod4g,

    /// iPod (5th Generation)
    Ipod5g,

    /// iPod mini (1st Generation)
    IpodMini1g,

    /// iPod mini (2nd Generation)
    IpodMini2g,

    /// iPod color
    IpodColor,

    /// iPod photo
    IpodPhoto,

    /// iPod Nano (1st Generation)
    IpodNano1g,

    /// Unknown Portable Media Player
    Unknown,
}

impl PMPModel {
	pub fn from_gestalt(gestalt: u32) -> PMPModel {
		use PMPModel::*;

		// Sourced from: http://www.ipodlinux.org/Generations/
		match gestalt {
			0x00010000 | 0x00010001 | 0x00010002 => Ipod1g,
			0x00020000 | 0x00020001 => Ipod2g,
			0x00030001 => Ipod3g,
			0x00050013 | 0x00050014 => Ipod4g,
			0x000B0005 | 0x000B0010 => Ipod5g,
			0x00040013 => IpodMini1g,
			0x00070002 => IpodMini2g,
			0x000C0005 | 0x000C0006 => IpodNano1g,
			0x00060000 => IpodPhoto,
			0x00060004 => IpodColor,
			_ => Unknown,
		}
	}
}
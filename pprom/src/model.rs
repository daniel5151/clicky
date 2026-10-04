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
			0x00040012 | 0x00040013 => IpodMini1g,
			0x00070002 => IpodMini2g,
			0x000C0005 | 0x000C0006 => IpodNano1g,
			0x00060000 => IpodPhoto,
			0x00060004 => IpodColor,
			_ => Unknown,
		}
	}

    pub fn model_name(self) -> &'static str {
        match self {
            PMPModel::Ipod1g => "iPod (1st generation)",
            PMPModel::Ipod2g => "iPod (2nd generation)",
            PMPModel::Ipod3g => "iPod (3rd generation)",
            PMPModel::Ipod4g => "iPod (4th generation)",
            PMPModel::Ipod5g => "iPod (5th generation)",
            PMPModel::IpodMini1g => "iPod mini (1st generation)",
            PMPModel::IpodMini2g => "iPod mini (2nd generation)",
            PMPModel::IpodColor => "iPod color",
            PMPModel::IpodPhoto => "iPod photo",
            PMPModel::IpodNano1g => "iPod nano (1st generation)",
            PMPModel::Unknown => "Unknown PMP",
        }
    }

}

impl std::fmt::Display for PMPModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.model_name())
    }
}

impl std::str::FromStr for PMPModel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ipod1g" => Ok(PMPModel::Ipod1g),
            "ipod2g" => Ok(PMPModel::Ipod2g),
            "ipod3g" => Ok(PMPModel::Ipod3g),
            "ipod4g" => Ok(PMPModel::Ipod4g),
            "ipod5g" => Ok(PMPModel::Ipod5g),
            "ipodmini1g" => Ok(PMPModel::IpodMini1g),
            "ipodmini2g" => Ok(PMPModel::IpodMini2g),
            "ipodcolor" => Ok(PMPModel::IpodColor),
            "ipodphoto" => Ok(PMPModel::IpodPhoto),
            "ipodnano1g" => Ok(PMPModel::IpodNano1g),
            _ => Err(format!(
                "unknown model {:?}; valid values: ipod1g, ipod2g, ipod3g, ipod4g, ipod5g, \
                 ipodmini1g, ipodmini2g, ipodcolor, ipodphoto, ipodnano1g",
                s
            )),
        }
    }
}
mod model;
mod syscfg;

pub use crate::model::PMPModel;
pub use crate::syscfg::{Record, SysCfg};

#[derive(Debug, Clone)]
pub struct Rom {
    syscfg: Option<SysCfg>,
    contents: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RomError {}

impl std::fmt::Display for RomError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {}
    }
}

impl std::error::Error for RomError {}

impl Default for Rom {
    fn default() -> Rom {
        Rom::new()
    }
}

impl Rom {
    pub fn new() -> Rom {
        Rom {
            syscfg: None,
            contents: Vec::new(),
        }
    }

    pub fn from_dump(dump: &[u8]) -> Result<Rom, RomError> {
        let syscfg = SysCfg::from_rom(dump);

        Ok(Rom {
            syscfg,
            contents: dump.to_vec(),
        })
    }

    pub fn model(&self) -> PMPModel {
        // A dump we couldn't find a table in tells us nothing about the model.
        let syscfg = match &self.syscfg {
            Some(syscfg) => syscfg,
            None => return PMPModel::Unknown,
        };

        // Nor does one whose table has no `HwVr` record.
        let hw_vr = match syscfg.get(b"HwVr") {
            Some(record) => record,
            None => return PMPModel::Unknown,
        };

        // The revision is the *second* word of the record, not the first.
        let gestalt = hw_vr.word(1);

        PMPModel::from_gestalt(gestalt)
    }

    pub fn syscfg(&self) -> Option<&SysCfg> {
        self.syscfg.as_ref()
    }

    pub fn contents(&self) -> &[u8] {
        &self.contents
    }
}

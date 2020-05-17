/// Copied from ipodloader2 source
#[allow(non_snake_case)]
#[repr(C)]
#[derive(Default)]
pub struct sysinfo_t {
    pub IsyS: u32, /* == "IsyS" */
    pub len: u32,
    pub BoardHwName: [u8; 16],
    pub pszSerialNumber: [u8; 32],
    pub pu8FirewireGuid: [u8; 16],
    pub boardHwRev: u32,
    pub bootLoaderImageRev: u32,
    pub diskModeImageRev: u32,
    pub diagImageRev: u32,
    pub osImageRev: u32,
    pub iram_perhaps: u32,
    pub Flsh: u32,
    pub flash_zero: u32,
    pub flash_base: u32,
    pub flash_size: u32,
    pub flash_zero2: u32,
    pub Sdrm: u32,
    pub sdram_zero: u32,
    pub sdram_base: u32,
    pub sdram_size: u32,
    pub sdram_zero2: u32,
    pub Frwr: u32,
    pub frwr_zero: u32,
    pub frwr_base: u32,
    pub frwr_size: u32,
    pub frwr_zero2: u32,
    pub Iram: u32,
    pub iram_zero: u32,
    pub iram_base: u32,
    pub iram_size: u32,
    pub iram_zero2: u32,
    pub pad7: [u32; 30],
    pub boardHwSwInterfaceRev: u32,
    /* added in V3
     * pub HddFirmwareRev: [u8; 10],
     * pub RegionCode: u16,
     * pub PolicyFlags: u32,
     * pub ModelNumStr: [u8; 16], */
}

impl sysinfo_t {
    pub fn as_slice(&self) -> &[u8] {
        // XXX: this will break on big-endian systems
        unsafe {
            std::slice::from_raw_parts(self as *const _ as *const u8, std::mem::size_of::<Self>())
        }
    }
}

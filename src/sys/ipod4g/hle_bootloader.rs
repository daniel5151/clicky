/// Copied from ipodloader2 source
#[allow(non_snake_case)]
#[repr(C, packed)]
#[derive(Copy, Clone, Default)]
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

// Safety:
// - All of `hd_driveid`'s fields are of type `uX` and/or arrays of `uX`, which
//   are Pod types themselves
// - `hd_driveid` is repr(C, packed), ensuring no padding
unsafe impl bytemuck::Zeroable for sysinfo_t {}
unsafe impl bytemuck::Pod for sysinfo_t {}

use crate::devices::prelude::*;

#[derive(Debug, Default)]
struct Dma {
    label: Option<&'static str>,

    cmd: u32,
    status: u32,
    ram_addr: u32,
    flags: u32,
    per_addr: u32,
    incr: u32,
}

impl Device for Dma {
    fn kind(&self) -> &'static str {
        "<dma>"
    }

    fn label(&self) -> Option<&'static str> {
        self.label
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x00 => "Cmd",
            0x04 => "Status",
            0x10 => "Ram Addr",
            0x14 => "Flags",
            0x18 => "Per Addr",
            0x1c => "Incr",
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

/// PP5020 DMA Engine
#[derive(Debug)]
pub struct DmaCon {
    dma: [Dma; 8],
    master_control: u32,
    master_status: u32,
    req_status: u32,

    // HACK: IDE DMA doesn't actually go through the DMA controller
    // that said, to keep things simple in the emulator, we route IDE DMA through the main DMA
    // engine...
    //
    // As per the pp5020 spec sheet: "A dedicated, high-performance ATA-66IDE controller with its
    // own DMA engine frees the processors from mundane management tasks."
    ide_dmarq: irq::Reciever,
}

impl DmaCon {
    pub fn new(ide_dmarq: irq::Reciever) -> DmaCon {
        let mut dma = DmaCon {
            dma: Default::default(),
            master_control: 0,
            master_status: 0,
            req_status: 0,

            ide_dmarq,
        };

        dma.dma[0].label = Some("0");
        dma.dma[1].label = Some("1");
        dma.dma[2].label = Some("2");
        dma.dma[3].label = Some("3");
        dma.dma[4].label = Some("4");
        dma.dma[5].label = Some("5");
        dma.dma[6].label = Some("6");
        dma.dma[7].label = Some("7");

        dma
    }

    /// XXX: remove this once DMA is properly sorted out
    pub fn do_ide_dma(&self) -> bool {
        self.ide_dmarq.asserted()
    }
}

impl Device for DmaCon {
    fn kind(&self) -> &'static str {
        "DMA Engine"
    }

    fn probe(&self, offset: u32) -> Probe {
        let reg = match offset {
            0x0 => "Master Control",
            0x4 => "Master Status",
            0x8 => "Req Status",
            0x1000..=0x10ff => {
                let id = (offset - 0x1000) / 0x20;
                return Probe::from_device(&self.dma[id as usize], offset % 0x20);
            }
            _ => return Probe::Unmapped,
        };

        Probe::Register(reg)
    }
}

impl Memory for DmaCon {
    fn r32(&mut self, offset: u32) -> MemResult<u32> {
        match offset {
            0x0 => Err(StubRead(Error, self.master_control)),
            0x4 => Err(StubRead(Error, self.master_status)),
            0x8 => Err(StubRead(Error, self.req_status)),
            0x1000..=0x10ff => {
                let id = (offset - 0x1000) / 0x20;
                let dma = &mut self.dma[id as usize];
                match offset % 0x20 {
                    0x00 => Err(StubRead(Error, dma.cmd)),
                    0x04 => Err(StubRead(Error, dma.status)),
                    0x10 => Err(StubRead(Error, dma.ram_addr)),
                    0x14 => Err(StubRead(Error, dma.flags)),
                    0x18 => Err(StubRead(Error, dma.per_addr)),
                    0x1c => Err(StubRead(Error, dma.incr)),
                    _ => Err(Unexpected),
                }
            }
            _ => Err(Unexpected),
        }
    }

    fn w32(&mut self, offset: u32, val: u32) -> MemResult<()> {
        match offset {
            0x0 => Err(StubWrite(Error, self.master_control = val)),
            0x4 => Err(StubWrite(Error, self.master_status = val)),
            0x8 => Err(StubWrite(Error, self.req_status = val)),
            0x1000..=0x10ff => {
                let id = (offset - 0x1000) / 0x20;
                let dma = &mut self.dma[id as usize];
                match offset % 0x20 {
                    0x00 => Err(StubWrite(Error, dma.cmd = val)),
                    0x04 => Err(StubWrite(Error, dma.status = val)),
                    0x10 => Err(StubWrite(Error, dma.ram_addr = val)),
                    0x14 => Err(StubWrite(Error, dma.flags = val)),
                    0x18 => Err(StubWrite(Error, dma.per_addr = val)),
                    0x1c => Err(StubWrite(Error, dma.incr = val)),
                    _ => Err(Unexpected),
                }
            }
            _ => Err(Unexpected),
        }
    }
}

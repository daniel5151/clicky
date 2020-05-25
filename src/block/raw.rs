use std::fs::File;

#[derive(Debug)]
pub struct RawBlockDev {
    file: File,
}

impl RawBlockDev {
    pub fn new(file: File) -> RawBlockDev {
        RawBlockDev { file }
    }
}

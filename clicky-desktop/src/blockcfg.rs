use std::str::FromStr;

/// Helper struct to parse Block Device configurations.
pub enum BlockCfg {
    /// `null:len=<len>`
    Null { len: u64 },
    /// `raw:file=/path/`
    Raw { path: String },
    /// `mem:file=/path/[,truncate=<len>]`
    Mem { path: String, truncate: Option<u64> },
}

fn parse_capacity(desc: &str) -> Option<u64> {
    use human_size::{Byte, ParsingError, Size, SpecificSize};
    match desc.parse::<Size>() {
        Ok(s) => {
            let bytes: SpecificSize<Byte> = s.into();
            Some(bytes.value() as u64)
        }
        Err(ParsingError::MissingMultiple) => desc.parse::<u64>().ok(),
        Err(_) => None,
    }
}

impl FromStr for BlockCfg {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<BlockCfg, &'static str> {
        let mut s = s.splitn(2, ':');
        let kind = s.next().unwrap();
        Ok(match kind {
            "null" => {
                let s = s.next().ok_or("missing required options")?.split(',');

                let mut len = None;

                for arg in s {
                    let mut s = arg.split('=');
                    let kind = s.next().unwrap();
                    match kind {
                        "len" => {
                            len = Some(
                                parse_capacity(s.next().ok_or("missing argument for `len`")?)
                                    .ok_or("could not parse `len`")?,
                            );
                        }
                        _ => return Err("unknown `null` option"),
                    }
                }

                BlockCfg::Null {
                    len: len.ok_or("missing `len` parameter")?,
                }
            }
            "raw" => {
                let s = s.next().ok_or("missing required options")?.split(',');

                let mut file = None;

                for arg in s {
                    let mut s = arg.split('=');
                    let kind = s.next().unwrap();
                    match kind {
                        "file" => {
                            file = Some(s.next().ok_or("missing argument for `file`")?.into())
                        }
                        _ => return Err("unknown `len` option"),
                    }
                }

                BlockCfg::Raw {
                    path: file.ok_or("missing `file` parameter")?,
                }
            }
            "mem" => {
                let s = s.next().ok_or("missing required options")?.split(',');

                let mut file = None;
                let mut truncate = None;

                for arg in s {
                    let mut s = arg.split('=');
                    let kind = s.next().unwrap();
                    match kind {
                        "file" => {
                            file = Some(s.next().ok_or("missing argument for `file`")?.into())
                        }
                        "truncate" => {
                            truncate = Some(
                                parse_capacity(s.next().ok_or("missing argument for `truncate`")?)
                                    .ok_or("could not parse `truncate`")?,
                            )
                        }
                        _ => return Err("unknown `len` option"),
                    }
                }

                BlockCfg::Mem {
                    path: file.ok_or("missing `file` parameter")?,
                    truncate,
                }
            }
            _ => return Err("invalid block kind"),
        })
    }
}

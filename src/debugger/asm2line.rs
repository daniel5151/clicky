//! Parse the output of `objdump -ld --prefix-addresses <bin>`
//!
//! this code is a quick and dirty implementation that's hardly robust and not
//! at all efficient. it really should be thrown out...
//!
//! don't judge me

// TODO: burn this to the ground and write something that doesn't suck
// Heck, it'll probably be a lot cleanr to just write a gdb stub, instead of
// clobbering together this janky system

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Debug)]
pub struct LineInfo {
    disasm: String,
    function: String,
    file_line: Option<(String, usize)>,
}

#[derive(Debug)]
pub struct Asm2Line {
    file_content: HashMap<String, Vec<String>>,
    map: HashMap<u32, LineInfo>,
}

enum ParseState {
    Start,
    Disasm {
        cur_file_ln: Option<(String, usize)>,
    },
}

impl Asm2Line {
    pub fn new() -> Asm2Line {
        Asm2Line {
            file_content: HashMap::new(),
            map: HashMap::new(),
        }
    }

    pub fn load_objdump(&mut self, objdump_fname: &str) -> std::io::Result<()> {
        let file = BufReader::new(File::open(&objdump_fname)?);

        let lines = file.lines();

        let mut state = ParseState::Start;

        for ln in lines {
            let ln = ln?.replace('\t', " ");
            match state {
                ParseState::Start => {
                    // sift through the crud at the start
                    if ln.starts_with("Disassembly of section") {
                        state = ParseState::Disasm { cur_file_ln: None }
                    }
                }
                ParseState::Disasm {
                    ref mut cur_file_ln,
                } => {
                    if ln.ends_with("():") {
                        // new function name
                        *cur_file_ln = None
                    } else if ln.starts_with('/') {
                        // file + line number
                        let file_lnnum = ln.split(':').collect::<Vec<_>>();

                        let file = file_lnnum[0].to_string();
                        let lnnum = file_lnnum[1].parse().unwrap();

                        self.file_content.insert(file.clone(), Vec::new());
                        *cur_file_ln = Some((file, lnnum));
                    } else if ln.starts_with(" ...") {
                        // repeat above
                    } else {
                        // memory location disasm
                        let func_disasm = ln[8..].splitn(2, '>').collect::<Vec<_>>();

                        let addr = u32::from_str_radix(&ln[..8], 16).unwrap();
                        let function = func_disasm[0][1..].to_owned() + ">";
                        let disasm = func_disasm[1].to_owned();

                        self.map.insert(
                            addr,
                            LineInfo {
                                disasm,
                                function,
                                file_line: cur_file_ln.clone(),
                            },
                        );
                    }
                }
            }
        }

        // additional pass to get the actual source code line
        for (fname, content) in self.file_content.iter_mut() {
            let file = BufReader::new(File::open(&fname)?);
            *content = (file.lines().collect::<Result<Vec<_>, _>>())?;
        }

        Ok(())
    }

    pub fn lookup(&self, addr: u32) -> Option<String> {
        match self.map.get(&addr) {
            Some(info) => Some(match &info.file_line {
                Some((file, line)) => format!(
                    "{:#010x}: {:50}\n{}\n{}\n{}",
                    addr,
                    info.disasm,
                    format!("{}:{}", file, line),
                    info.function,
                    {
                        let mut s = String::new();
                        const CONTEXT: usize = 4;
                        for ln in (i32::max(0, (*line - 1) as i32 - CONTEXT as i32) as usize)
                            ..usize::min(self.file_content[file].len(), line + CONTEXT)
                        {
                            if (*line - 1) == ln {
                                s += ">"
                            } else {
                                s += " "
                            }
                            s += &(self.file_content[file][ln].clone() + "\n")
                        }
                        s
                    }
                ),
                None => format!("{:#010x}: {:50}\n{:25}", addr, info.disasm, info.function),
            }),
            None => None,
        }
    }
}

#[macro_use]
extern crate static_assertions;

#[macro_use]
extern crate log;

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub type DynResult<T> = Result<T, Box<dyn std::error::Error>>;

use structopt::StructOpt;

pub mod block;
pub mod devices;
pub mod error;
pub mod gui;
pub mod memory;
pub mod signal;
pub mod sys;

mod gdb;
use crate::gdb::{make_gdbstub, GdbCfg};

use crate::block::{BlockCfg, BlockDev};
use crate::sys::ipod4g::{BootKind, Ipod4g, Ipod4gControls, Ipod4gGdb};

const SYSDUMP_FILENAME: &str = "sysdump.log";

#[derive(StructOpt)]
#[structopt(name = "clicky")]
#[structopt(about = r#"
An emulator for the classic clickwheel iPod 4g.
"#)]
struct Args {
    /// Firmware file to load
    #[structopt(parse(from_os_str))]
    firmware: PathBuf,

    /// HDD image to use
    ///
    /// At the moment, this should most likely be set to
    /// `--hdd=raw:/path/to/ipodhd.img`.
    #[structopt(long)]
    hdd: BlockCfg,

    /// Flash ROM binary to use.
    #[structopt(long)]
    flash_rom: Option<PathBuf>,

    /// Spawn a GDB server at system startup.
    ///
    /// Format: `-g <port/path>[,on-fatal-err[,and-on-start]]`
    ///
    /// If "on-fatal-err" is provided, the GDB server will only launch if the
    /// system experiences a fatal error. Providing "and-on-start" will also
    /// launch the GDB server at system startup.
    ///
    /// e.g: `-g 9001,on-fatal-err` will spawn a gdb server listening on port
    /// 9001 if the system experiences a fatal error, `-g /tmp/clicky`
    /// creates a new Unix Domain Socket at `/tmp/clicky`, and waits for a GDB
    /// connection before starting execution.
    #[structopt(short, long)]
    gdb: Option<GdbCfg>,
}

enum System {
    Bare(Ipod4g),
    Debug { system_gdb: Ipod4gGdb, cfg: GdbCfg },
}

impl core::ops::Deref for System {
    type Target = Ipod4g;

    fn deref(&self) -> &Ipod4g {
        use self::System::*;
        match self {
            Bare(sys) => sys,
            Debug { system_gdb, .. } => system_gdb.sys_ref(),
        }
    }
}

impl core::ops::DerefMut for System {
    fn deref_mut(&mut self) -> &mut Ipod4g {
        use self::System::*;
        match self {
            Bare(sys) => sys,
            Debug { system_gdb, .. } => system_gdb.sys_mut(),
        }
    }
}

fn main() -> DynResult<()> {
    pretty_env_logger::formatted_builder()
        .filter(None, log::LevelFilter::Debug)
        .filter(Some("armv4t_emu"), log::LevelFilter::Debug)
        .parse_filters(&std::env::var("RUST_LOG").unwrap_or_default())
        .init();

    let args = Args::from_args();

    let hdd: Box<dyn BlockDev> = match args.hdd {
        BlockCfg::Null { len } => Box::new(block::backend::Null::new(len)),
        BlockCfg::Raw { path } => {
            let file = fs::OpenOptions::new().read(true).write(true).open(path)?;
            Box::new(block::backend::Raw::new(file))
        }
    };

    let boot_kind = match args.flash_rom {
        Some(path) => BootKind::ColdBoot {
            flash_rom: fs::read(path)?,
        },
        None => BootKind::HLEBoot {
            fw_file: fs::File::open(args.firmware)?,
        },
    };

    let mut system = Ipod4g::new(hdd, boot_kind)?;

    // hook-up controls
    let minifb_controls = {
        use devices::platform::pp::KeypadSignals;
        use minifb::Key;

        let Ipod4gControls {
            mut hold,
            keypad:
                KeypadSignals {
                    mut action,
                    mut up,
                    mut down,
                    mut left,
                    mut right,
                },
        } = system.take_controls().unwrap();

        // set hold high by default
        hold.set_high();

        let mut controls: HashMap<_, gui::KeyCallback> = HashMap::new();
        controls.insert(
            Key::H, // H for Hold
            Box::new(move |pressed| {
                if pressed {
                    // toggle on and off
                    match hold.is_set_high() {
                        false => hold.set_high(),
                        true => hold.set_low(),
                    }
                }
            }),
        );

        macro_rules! connect_keypad_btn {
            ($key:expr, $signal:expr) => {
                controls.insert(
                    $key,
                    Box::new(move |pressed| {
                        if pressed {
                            $signal.assert()
                        } else {
                            $signal.clear()
                        }
                    }),
                );
            };
        }

        connect_keypad_btn!(Key::Up, up);
        connect_keypad_btn!(Key::Down, down);
        connect_keypad_btn!(Key::Left, left);
        connect_keypad_btn!(Key::Right, right);
        connect_keypad_btn!(Key::Enter, action);

        controls
    };

    // spawn the UI thread
    let _minifb_ui =
        gui::minifb::IPodMinifb::new((160, 128), system.render_callback(), minifb_controls);

    let mut system = match args.gdb {
        Some(cfg) => System::Debug {
            system_gdb: Ipod4gGdb::new(system),
            cfg,
        },
        None => System::Bare(system),
    };

    let mut debugger = None;

    let system_result = match &mut system {
        System::Bare(system) => system.run(),
        System::Debug { system_gdb, cfg } => {
            // check if a debugger should be connected at boot
            if cfg.on_start {
                debugger = Some(make_gdbstub(cfg.clone())?)
            }

            match debugger {
                None => system.run(),
                // hand off control to the debugger
                Some(ref mut debugger) => match debugger.run(system_gdb) {
                    Ok(dc_reason) => {
                        eprintln!("Disconnected from GDB: {:?}", dc_reason);

                        use gdbstub::DisconnectReason;
                        match dc_reason {
                            DisconnectReason::Disconnect => {
                                eprintln!("Target is still running. Resuming execution...");
                                system.run()
                            }
                            DisconnectReason::TargetHalted => {
                                eprintln!("Target halted!");
                                Ok(())
                            }
                            DisconnectReason::Kill => {
                                eprintln!("GDB sent a kill command!");
                                return Ok(());
                            }
                        }
                    }
                    Err(gdbstub::GdbStubError::TargetError(e)) => Err(e),
                    Err(e) => return Err(e.into()),
                },
            }
        }
    };

    if let Err(fatal_error) = system_result {
        eprintln!("Fatal Error! Caused by: {:#010x?}", fatal_error);
        eprintln!("Dumping system state to {}", SYSDUMP_FILENAME);
        std::fs::write(SYSDUMP_FILENAME, format!("{:#x?}", *system))?;

        match &mut system {
            System::Bare(_system) => {}
            System::Debug { system_gdb, cfg } => {
                if cfg.on_fatal_err {
                    system_gdb.sys_mut().freeze();

                    if debugger.is_none() {
                        debugger = Some(make_gdbstub(cfg.clone())?)
                    }

                    match debugger.unwrap().run(system_gdb) {
                        Ok(_) => eprintln!("Disconnected from post-mortem GDB session."),
                        Err(e) => return Err(e.into()),
                    }
                }
            }
        };
    }

    Ok(())
}

#[macro_use]
extern crate static_assertions;

#[macro_use]
extern crate log;

use std::collections::HashMap;
use std::error::Error as StdError;
use std::fs;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;

use structopt::StructOpt;

pub mod block;
pub mod devices;
pub mod gui;
pub mod memory;
pub mod signal;
pub mod sys;
pub mod util;

use crate::block::{BlockCfg, BlockDev};
use crate::sys::ipod4g::{BootKind, Ipod4g, Ipod4gControls};

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
    /// At the moment, this should be `--hdd=raw:/path/to/ipodhd.img`
    #[structopt(long)]
    hdd: BlockCfg,

    /// Flash ROM binary to use
    #[structopt(long)]
    flash_rom: Option<PathBuf>,

    /// spawn a gdb server listening on the specified port
    #[structopt(short)]
    gdbport: Option<u16>,

    /// spawn a gdb server if the guest triggers a fatal error
    #[structopt(long, requires("gdbport"))]
    gdb_fatal_err: bool,
}

fn new_tcp_gdbstub<T: gdbstub::Target>(
    port: u16,
) -> Result<gdbstub::GdbStub<T, TcpStream>, std::io::Error> {
    let sockaddr = format!("127.0.0.1:{}", port);
    info!("Waiting for a GDB connection on {:?}...", sockaddr);

    let sock = TcpListener::bind(sockaddr)?;
    let (stream, addr) = sock.accept()?;
    info!("Debugger connected from {}", addr);

    Ok(gdbstub::GdbStub::new(stream))
}

fn main() -> Result<(), Box<dyn StdError>> {
    pretty_env_logger::formatted_builder()
        .filter(None, log::LevelFilter::Debug)
        .filter(Some("armv4t_emu"), log::LevelFilter::Debug)
        .parse_filters(&std::env::var("RUST_LOG").unwrap_or_default())
        .init();

    let args = Args::from_args();

    // TODO: properly expose HDD options to CLI
    let hdd: Box<dyn BlockDev> = match args.hdd {
        BlockCfg::Null { len } => Box::new(block::backend::Null::new(len)),
        BlockCfg::Raw { path } => {
            let file = fs::File::open(path)?;
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
        let Ipod4gControls { mut hold } = system.take_controls().unwrap();

        // set hold high by default
        hold.set_high();

        let mut controls: HashMap<_, gui::KeyCallback> = HashMap::new();
        controls.insert(
            minifb::Key::H, // H for Hold
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
        controls
    };

    // spawn the UI thread
    let _minifb_ui =
        gui::minifb::IPodMinifb::new((160, 128), system.render_callback(), minifb_controls);

    // check if a debugger should be connected at boot
    let debugger = match (args.gdb_fatal_err, args.gdbport) {
        (false, Some(port)) => Some(new_tcp_gdbstub(port)?),
        _ => None,
    };

    let system_result = match debugger {
        // hand off control to the debugger
        Some(mut debugger) => match debugger.run(&mut system) {
            Ok(state) => {
                eprintln!("Disconnected from GDB. Target state: {:?}", state);
                if state == gdbstub::TargetState::Running {
                    eprintln!("Target is still running. Resuming execution...");
                    system.run()
                } else {
                    Ok(())
                }
            }
            Err(gdbstub::Error::TargetError(e)) => Err(e),
            Err(e) => return Err(e.into()),
        },
        // just run the system until it finishes, or an error occurs
        None => system.run(),
    };

    if let Err(fatal_error) = system_result {
        eprintln!("Fatal Error! Caused by: {:#010x?}", fatal_error);
        eprintln!("Dumping system state to {}", SYSDUMP_FILENAME);
        std::fs::write(SYSDUMP_FILENAME, format!("{:#x?}", system))?;

        if args.gdb_fatal_err {
            let port = args
                .gdbport
                .expect("gbdport guaranteed to be present by structopt");
            let mut debugger = new_tcp_gdbstub(port)?;

            system.freeze();
            match debugger.run(&mut system) {
                Ok(_) => eprintln!("Disconnected from post-mortem GDB session."),
                Err(e) => return Err(e.into()),
            }
        }
    }

    Ok(())
}

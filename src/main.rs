use std::error::Error as StdError;
use std::fs;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;

use log::*;
use structopt::StructOpt;

pub mod devices;
pub mod memory;
pub mod sys;
pub mod util;

use crate::sys::ipod4g::Ipod4g;

#[derive(StructOpt)]
#[structopt(name = "clicky")]
#[structopt(about = r#"
An emulator for the classic clickwheel iPod 4g.
"#)]
struct Args {
    /// firmware file to load
    #[structopt(parse(from_os_str))]
    firmware: PathBuf,

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
        .filter(None, LevelFilter::Debug)
        .filter(Some("armv4t_emu"), LevelFilter::Debug)
        .parse_filters(&std::env::var("RUST_LOG").unwrap_or_default())
        .init();

    let args = Args::from_args();

    // create the base system
    let file = fs::File::open(args.firmware)?;
    let mut system = Ipod4g::new_hle(file)?;

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

        if args.gdb_fatal_err {
            let port = args
                .gdbport
                .expect("gbdport guaranteed to be present by structopt");
            let mut debugger = new_tcp_gdbstub(port)?;

            system.freeze();
            match debugger.run(&mut system) {
                Ok(_) => {
                    eprintln!("Disconnected from post-mortem GDB session.");
                    return Ok(());
                }
                Err(e) => return Err(e.into()),
            }
        } else {
            eprintln!("Dumping system state:");
            eprintln!("============");
            eprintln!("{:#010x?}", system);
            eprintln!("============");
        }

        return Err("fatal error".into());
    }

    Ok(())
}

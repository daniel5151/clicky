#[macro_use]
extern crate log;

use std::fs;
use std::io;
use std::path::PathBuf;

use structopt::StructOpt;
use thiserror::Error;

use pprom::Rom;

#[derive(Error, Debug)]
enum PpromError {
    #[error("couldn't read ROM dump {}: {source}", .path.display())]
    Read { path: PathBuf, source: io::Error },

    #[error("couldn't parse ROM dump: {0}")]
    Rom(#[from] pprom::RomError),

    #[error("no SCfg table in this dump")]
    NoSysCfg,

    #[error("couldn't format a hexdump: {0}")]
    Hexdump(#[from] io::Error),
}

#[derive(StructOpt)]
#[structopt(name = "pprom")]
#[structopt(about = "Explore PortalPlayer-based iPod InternalROM dumps.")]
struct Args {
    /// Path to a dumped InternalROM binary.
    #[structopt(parse(from_os_str))]
    rom: PathBuf,

    #[structopt(subcommand)]
    cmd: Option<Command>,
}

#[derive(StructOpt)]
enum Command {
    /// Summarize the ROM. This is the default when no subcommand is given.
    Info,
    /// Dump every key in the ROM's `SCfg` table.
    Keys,
}

fn main() {
    pretty_env_logger::init();

    if let Err(e) = run() {
        eprintln!("error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), PpromError> {
    let args = Args::from_args();

    let dump = fs::read(&args.rom).map_err(|source| PpromError::Read {
        path: args.rom.clone(),
        source,
    })?;
    debug!("read {} bytes from {}", dump.len(), args.rom.display());

    let rom = Rom::from_dump(&dump)?;

    match args.cmd.unwrap_or(Command::Info) {
        Command::Info => cmd_info(&rom),
        Command::Keys => cmd_keys(&rom),
    }
}

fn cmd_info(rom: &Rom) -> Result<(), PpromError> {
    println!("model:  {:?}", rom.model());
    println!("length: {} bytes", rom.contents().len());

    match rom.syscfg() {
        Some(cfg) => println!("SCfg:   {} keys", cfg.records.len()),
        None => println!("SCfg:   not found"),
    }

    Ok(())
}

fn cmd_keys(rom: &Rom) -> Result<(), PpromError> {
    let cfg = rom.syscfg().ok_or(PpromError::NoSysCfg)?;

    println!(
        "SCfg v{}.{}: {} keys",
        cfg.version.0,
        cfg.version.1,
        cfg.records.len(),
    );
    println!();

    for r in &cfg.records {
        println!("key: {}", r.tag_str());
        let mut buf = Vec::new();
        hxdmp::hexdump(&r.value, &mut buf)?;
        println!("{}", String::from_utf8_lossy(&buf));
    }

    Ok(())
}

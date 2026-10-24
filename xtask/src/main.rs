//! Clicky repo-specific automation.
//!
//! Follows the xtask workflow/convention, as described at
//! <https://github.com/matklad/cargo-xtask>

#![forbid(unsafe_code)]

use clap::Parser;
use clap::Subcommand;
use std::path::Path;
use std::path::PathBuf;

pub mod logger;
pub mod tasks;

/// Common context passed into every Xtask
#[derive(Clone)]
pub struct XtaskCtx {
    /// Project root directory
    pub root: PathBuf,
}

/// Common trait implemented by all Xtask subcommands.
pub trait Xtask: Parser {
    /// Run the Xtask.
    ///
    /// For consistency and simplicity, `Xtask` implementations are allowed to
    /// assume that they are being run from the root of the repo's filesystem.
    /// Callers of `Xtask::run` should take care to ensure
    /// [`std::env::set_current_dir`] was called prior to invoking `Xtask::run`.
    fn run(self, ctx: XtaskCtx) -> anyhow::Result<()>;
}

#[derive(Parser)]
#[clap(name = "xtask", about = "clicky repo automation")]
struct Cli {
    #[clap(subcommand)]
    command: Commands,

    /// Specify a custom project root directory.
    #[clap(long)]
    custom_root: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    Ci(tasks::Ci),
    Fmt(tasks::Fmt),
    Check(tasks::Check),
    Clippy(tasks::Clippy),
    Test(tasks::Test),
}

fn main() {
    logger::init();

    if let Err(e) = try_main() {
        log::error!("{:#}", e);
        std::process::exit(1);
    }
}

fn try_main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let orig_root = Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(1)
        .unwrap()
        .to_path_buf();

    let root = cli
        .custom_root
        .map(std::path::absolute)
        .transpose()?
        .unwrap_or(orig_root);

    // for consistency, always run xtasks as though they were run from the root
    std::env::set_current_dir(&root)?;

    let ctx = XtaskCtx { root };

    match cli.command {
        Commands::Ci(task) => task.run(ctx),
        Commands::Fmt(task) => task.run(ctx),
        Commands::Check(task) => task.run(ctx),
        Commands::Clippy(task) => task.run(ctx),
        Commands::Test(task) => task.run(ctx),
    }
}

use crate::Xtask;
use crate::XtaskCtx;
use anyhow::Context;
use clap::Parser;
use xshell::{Shell, cmd};

/// Run cargo check across the workspace
#[derive(Parser, Debug, Default)]
#[clap(about = "Run cargo check across the workspace")]
pub struct Check;

impl Xtask for Check {
    fn run(self, _ctx: XtaskCtx) -> anyhow::Result<()> {
        let sh = Shell::new()?;
        sh.set_var("RUSTFLAGS", "-Dwarnings");
        cmd!(sh, "cargo check --workspace --all-targets --all-features")
            .run()
            .context("cargo check failed")?;
        Ok(())
    }
}

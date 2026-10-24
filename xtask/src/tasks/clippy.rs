use crate::Xtask;
use crate::XtaskCtx;
use anyhow::Context;
use clap::Parser;
use xshell::{Shell, cmd};

/// Run cargo clippy across the workspace
#[derive(Parser, Debug, Default)]
#[clap(about = "Run cargo clippy across the workspace")]
pub struct Clippy;

impl Xtask for Clippy {
    fn run(self, _ctx: XtaskCtx) -> anyhow::Result<()> {
        let sh = Shell::new()?;
        cmd!(
            sh,
            "cargo clippy --workspace --all-targets --all-features -- -D warnings"
        )
        .run()
        .context("cargo clippy failed")?;
        Ok(())
    }
}

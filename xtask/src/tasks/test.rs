use crate::Xtask;
use crate::XtaskCtx;
use anyhow::Context;
use clap::Parser;
use xshell::{Shell, cmd};

/// Run cargo test across the workspace
#[derive(Parser, Debug, Default)]
#[clap(about = "Run cargo test across the workspace")]
pub struct Test;

impl Xtask for Test {
    fn run(self, _ctx: XtaskCtx) -> anyhow::Result<()> {
        let sh = Shell::new()?;
        cmd!(sh, "cargo test --workspace --all-targets --all-features")
            .run()
            .context("cargo test failed")?;
        cmd!(sh, "cargo test --workspace --doc --all-features")
            .run()
            .context("cargo test (doc) failed")?;
        Ok(())
    }
}

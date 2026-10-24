use crate::Xtask;
use crate::XtaskCtx;
use anyhow::Context;
use clap::Parser;
use xshell::{Shell, cmd};

/// Check or fix formatting with rustfmt
#[derive(Parser, Debug, Default)]
#[clap(about = "Check or fix formatting with rustfmt")]
pub struct Fmt {
    /// Format files instead of checking
    #[clap(long)]
    pub fix: bool,
}

impl Xtask for Fmt {
    fn run(self, _ctx: XtaskCtx) -> anyhow::Result<()> {
        let sh = Shell::new()?;
        let check: &[&str] = if self.fix { &[] } else { &["--", "--check"] };
        cmd!(sh, "cargo fmt --all {check...}")
            .run()
            .context("cargo fmt failed")?;
        Ok(())
    }
}

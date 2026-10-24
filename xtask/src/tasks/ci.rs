use crate::Xtask;
use crate::XtaskCtx;
use crate::tasks::{Clippy, Fmt, Test};
use clap::Parser;

/// Run CI checks (fmt, clippy, test)
#[derive(Parser, Debug, Default)]
#[clap(about = "Run CI checks (fmt, clippy, test)")]
pub struct Ci {
    /// Only run cargo fmt check
    #[clap(long)]
    pub fmt: bool,

    /// Only run cargo clippy
    #[clap(long)]
    pub clippy: bool,

    /// Only run cargo test
    #[clap(long)]
    pub test: bool,
}

impl Xtask for Ci {
    fn run(self, ctx: XtaskCtx) -> anyhow::Result<()> {
        let run_all = !self.fmt && !self.clippy && !self.test;
        let mut failed = false;

        if run_all || self.fmt {
            log::info!("=== cargo fmt ===");
            if let Err(e) = (Fmt { fix: false }).run(ctx.clone()) {
                log::error!("{:#}", e);
                failed = true;
            }
        }

        if run_all || self.clippy {
            log::info!("=== cargo clippy ===");
            if let Err(e) = Clippy.run(ctx.clone()) {
                log::error!("{:#}", e);
                failed = true;
            }
        }

        if run_all || self.test {
            log::info!("=== cargo test ===");
            if let Err(e) = Test.run(ctx.clone()) {
                log::error!("{:#}", e);
                failed = true;
            }
        }

        if failed {
            anyhow::bail!("one or more CI checks failed");
        }

        log::info!("==========================================");
        log::info!("  All checks passed!");
        log::info!("==========================================");

        Ok(())
    }
}

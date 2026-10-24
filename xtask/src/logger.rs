//! Runtime-selected logger supporting GitHub Actions and standard terminal output.

use log::{Level, LevelFilter, Log, Metadata, Record};
use owo_colors::OwoColorize;

struct GitHubActionsLogger {
    level: LevelFilter,
}

impl Log for GitHubActionsLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }

        match record.level() {
            Level::Error => eprintln!("::error::{}", record.args()),
            Level::Warn => eprintln!("::warning::{}", record.args()),
            Level::Info => println!("{}", record.args()),
            Level::Debug | Level::Trace => eprintln!("::debug::{}", record.args()),
        }
    }

    fn flush(&self) {}
}

struct TerminalLogger {
    level: LevelFilter,
}

impl Log for TerminalLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }

        match record.level() {
            Level::Error => eprintln!("{}: {}", "Error".red().bold(), record.args().red()),
            Level::Warn => eprintln!("{}: {}", "Warning".yellow().bold(), record.args().yellow()),
            Level::Info => println!("{}", record.args()),
            Level::Debug | Level::Trace => {
                eprintln!("{}: {}", "[debug]".cyan(), record.args())
            }
        }
    }

    fn flush(&self) {}
}

/// Initialize the logger, selecting the implementation at runtime.
pub fn init() {
    let in_gh_actions = std::env::var("GITHUB_ACTIONS").is_ok_and(|v| v == "true");

    let max_level = if std::env::var("XTASK_DEBUG").is_ok() {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    let logger: Box<dyn Log> = if in_gh_actions {
        Box::new(GitHubActionsLogger { level: max_level })
    } else {
        Box::new(TerminalLogger { level: max_level })
    };

    let _ = log::set_boxed_logger(logger);
    log::set_max_level(max_level);
}

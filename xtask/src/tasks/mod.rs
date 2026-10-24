//! Implementations of various Xtasks

mod check;
mod ci;
mod clippy;
mod fmt;
mod test;

pub use self::check::Check;
pub use self::ci::Ci;
pub use self::clippy::Clippy;
pub use self::fmt::Fmt;
pub use self::test::Test;

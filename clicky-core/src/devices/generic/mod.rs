//! Platform-agnostic devices.

pub mod asanram;
pub mod ide;
pub mod ram;
pub mod stub;

pub use asanram::*;
pub use ide::*;
pub use ram::*;
pub use stub::*;

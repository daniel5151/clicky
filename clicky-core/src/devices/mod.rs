//! Peripheral devices.

#![allow(
    clippy::unit_arg,  // substantially reduces boilerplate
    clippy::match_bool, // can make things more clear at times
    clippy::new_without_default, // just adds more boilerplate
)]

pub mod prelude;

pub mod display;
pub mod generic;
pub mod i2c;
pub mod platform;
pub mod util;

/// Common trait implemented by all emulated devices.
pub trait Device: Send + Sync {
    /// The name of the emulated device.
    fn kind(&self) -> &'static str;

    /// A descriptive label for a particular instance of the device
    /// (if applicable).
    fn label(&self) -> Option<&'static str> {
        None
    }

    /// Query what devices exist at a particular memory offset.
    fn probe(&self, offset: u32) -> Probe;
}

macro_rules! impl_devfwd {
    ($type:ty) => {
        impl Device for $type {
            fn kind(&self) -> &'static str {
                (**self).kind()
            }

            fn label(&self) -> Option<&'static str> {
                (**self).label()
            }

            fn probe(&self, offset: u32) -> Probe {
                (**self).probe(offset)
            }
        }
    };
}

impl_devfwd!(Box<dyn Device>);
impl_devfwd!(&dyn Device);
impl_devfwd!(&mut dyn Device);

/// A link in a chain of devices corresponding to a particular memory offset.
pub enum Probe {
    /// Branch node representing a device.
    Device {
        kind: &'static str,
        label: Option<&'static str>,
        next: Box<Probe>,
    },
    /// Leaf node representing a register.
    Register(&'static str),
    /// Unmapped memory.
    Unmapped,
}

impl Probe {
    // Convenience method to construct a `Probe::Device`
    pub fn from_device(device: &impl Device, offset: u32) -> Probe {
        Probe::Device {
            kind: device.kind(),
            label: device.label(),
            next: Box::new(device.probe(offset)),
        }
    }
}

impl std::fmt::Display for Probe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Probe::Device { kind, label, next } => {
                match label {
                    Some(label) => write!(f, "{}:{}", kind, label)?,
                    None => write!(f, "{}", kind)?,
                };

                match &**next {
                    Probe::Unmapped => {}
                    next => write!(f, " > {}", next)?,
                }
            }
            Probe::Register(name) => write!(f, "{}", name)?,
            Probe::Unmapped => write!(f, "<unmapped>")?,
        }

        Ok(())
    }
}

mod autogen;
mod macros;
mod reflection;

pub use autogen::*;
pub use reflection::Reflection;

pub mod prelude {
    pub use crate::autogen::*;
    pub use crate::reflection::Reflection;
    pub use crate::Privacy;
}

/// A type that represents the privacy level of modules, functions, or variables.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Privacy {
    Public = 0,
    Private = 1,
}

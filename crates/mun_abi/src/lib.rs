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

#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Privacy {
    Public = 0,
    Private = 1,
}

//! The Mun ABI
//!
//! The Mun ABI defines the binary format used to communicate between the Mun Compiler and Mun
//! Runtime.
#![warn(missing_docs)]

// Bindings are automatically generated from C on `cargo build`
mod autogen;

mod autogen_impl;
mod macros;
mod reflection;

pub use autogen::*;
pub use reflection::Reflection;

/// The Mun ABI prelude
///
/// The *prelude* contains imports that are used almost every time.
pub mod prelude {
    pub use crate::autogen::*;
    pub use crate::reflection::Reflection;
    pub use crate::Privacy;
}

/// A type that represents the privacy level of modules, functions, or variables.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Privacy {
    /// Publicly (and privately) accessible
    Public = 0,
    /// Privately accessible
    Private = 1,
}

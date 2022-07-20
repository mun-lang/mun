//! The Mun Runtime C API
//!
//! The Mun Runtime C API exposes runtime functionality using the C ABI. This can be used to
//! integrate the Mun Runtime into other languages that allow interoperability with C.
#![warn(missing_docs)]

pub mod gc;
pub mod runtime;

pub mod function;

#[macro_use]
#[cfg(test)]
mod test_util;

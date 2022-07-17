//! The Mun Runtime C API
//!
//! The Mun Runtime C API exposes runtime functionality using the C ABI. This can be used to
//! integrate the Mun Runtime into other languages that allow interoperability with C.
#![warn(missing_docs)]

pub mod gc;
pub mod runtime;

pub mod field_info;
pub mod function_info;
pub mod struct_info;
pub mod type_info;

#[macro_use]
#[cfg(test)]
mod test_util;

use std::{ffi::CString, os::raw::c_char};

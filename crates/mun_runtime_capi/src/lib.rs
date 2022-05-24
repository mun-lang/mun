//! The Mun Runtime C API
//!
//! The Mun Runtime C API exposes runtime functionality using the C ABI. This can be used to
//! integrate the Mun Runtime into other languages that allow interoperability with C.
#![warn(missing_docs)]

#[macro_use]
mod handle;

pub mod error;
pub mod gc;
pub mod hub;
pub mod runtime;

pub mod field_info;
pub mod function_info;
pub mod struct_info;
pub mod type_info;

#[macro_use]
#[cfg(test)]
mod test_util;

use std::{ffi::CString, os::raw::c_char};

/// Deallocates a string that was allocated by the runtime.
///
/// # Safety
///
/// This function receives a raw pointer as parameter. Only when the argument is not a null pointer,
/// its content will be deallocated. Passing pointers to invalid data or memory allocated by other
/// processes, will lead to undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn mun_destroy_string(string: *const c_char) {
    if !string.is_null() {
        // Destroy the string
        let _string = CString::from_raw(string as *mut _);
    }
}

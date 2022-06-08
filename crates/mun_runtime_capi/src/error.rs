//! Exposes error reporting using the C ABI.

use std::{ffi::CString, os::raw::c_char, ptr};

#[repr(C)]
#[derive(Clone, Copy)]
/// A C-style handle to an error message.
///
/// If the handle contains a non-null pointer, an error occurred.
pub struct ErrorHandle(pub *const c_char);

impl ErrorHandle {
    /// Constructs an `ErrorHandle` from the specified error message.
    pub(crate) fn new<T: Into<Vec<u8>>>(error_message: T) -> Self {
        let error_message = CString::new(error_message).expect("Invalid error message");
        Self(CString::into_raw(error_message))
    }
}

impl Default for ErrorHandle {
    fn default() -> Self {
        Self(ptr::null())
    }
}

/// Destructs the error message corresponding to the specified handle.
///
/// # Safety
///
/// Only call this function on an ErrorHandle once.
#[no_mangle]
pub unsafe extern "C" fn mun_error_destroy(error: ErrorHandle) {
    if !error.0.is_null() {
        let _drop = CString::from_raw(error.0 as *mut c_char);
    }
}

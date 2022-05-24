//! Exposes error reporting using the C ABI.

use crate::{hub::HUB, typed_handle};
use std::{ffi::CString, hash::Hash, os::raw::c_char, ptr};

typed_handle!(ErrorHandle);

/// Destructs the error corresponding to `error_handle`.
#[no_mangle]
pub extern "C" fn mun_error_destroy(error_handle: ErrorHandle) {
    // If an error exists, destroy it
    let _error = HUB.errors.unregister(error_handle);
}

/// Retrieves the error message corresponding to `error_handle`. If the `error_handle` exists, a
/// valid `char` pointer is returned, otherwise a null-pointer is returned.
#[no_mangle]
pub extern "C" fn mun_error_message(error_handle: ErrorHandle) -> *const c_char {
    let errors = HUB.errors.get_data();
    let error = match errors.get(&error_handle) {
        Some(error) => error,
        None => return ptr::null(),
    };

    let message = format!("{}", error);
    CString::new(message).unwrap().into_raw() as *const _
}

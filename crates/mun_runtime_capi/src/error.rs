//! Exposes error reporting using the C ABI.

use std::ffi::CString;
use std::hash::Hash;
use std::ops::{ControlFlow};
use std::os::raw::c_char;
use std::ptr;

use crate::hub::HUB;
use crate::{Token, TypedHandle};

/// A C-style handle to an error.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Hash, Eq, PartialEq)]
pub struct ErrorHandle(Token);

impl TypedHandle for ErrorHandle {
    fn new(token: Token) -> Self {
        Self(token)
    }

    fn token(&self) -> Token {
        self.0
    }
}

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
        None => return ptr::null_mut(),
    };

    let message = format!("{}", error);
    CString::new(message).unwrap().into_raw() as *const _
}

use std::{ffi::CString, os::raw::c_char};

pub mod error;

pub use error::ErrorHandle;

/// Deallocates a string that was allocated by the runtime.
///
/// # Safety
///
/// This function receives a raw pointer as parameter. Only when the argument is not a null pointer,
/// its content will be deallocated. Passing pointers to invalid data or memory allocated by other
/// processes, will lead to undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn mun_string_destroy(string: *const c_char) {
    if !string.is_null() {
        // Destroy the string
        let _string = CString::from_raw(string as *mut _);
    }
}

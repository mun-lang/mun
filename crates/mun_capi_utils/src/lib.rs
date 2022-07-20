use std::ffi::CStr;
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

/// Tries to convert a C style string pointer to a CStr.
///
/// # Safety
///
/// The caller must provide a valid C string with a null terminator, whose content doesnt change
/// during the lifetime `'a`.
pub unsafe fn try_convert_c_string<'a>(string: *const c_char) -> Result<&'a str, &'static str> {
    if string.is_null() {
        return Err("null pointer");
    }

    match CStr::from_ptr::<'a>(string).to_str() {
        Ok(name) => Ok(name),
        Err(_) => Err("invalid UTF-8 encoded"),
    }
}

#[macro_export]
macro_rules! assert_getter1 {
    ($fun:ident ( $handle:expr, $($arg:ident),+ $(,)?)) => {
        $(
            let mut $arg = std::mem::MaybeUninit::uninit();
        )+
        assert!(unsafe { $fun($handle, $($arg.as_mut_ptr()),+) }.is_ok());
        $(
            let $arg = unsafe { $arg.assume_init() };
        )+
    };
}

#[macro_export]
macro_rules! assert_getter2 {
    ($fun:ident ( $handle:expr, $arg1:expr, $($arg:ident),+ $(,)?)) => {
        $(
            let mut $arg = std::mem::MaybeUninit::uninit();
        )+
        assert!(unsafe { $fun($handle, $arg1, $($arg.as_mut_ptr()),+) }.is_ok());
        $(
            let $arg = unsafe { $arg.assume_init() };
        )+
    };
}

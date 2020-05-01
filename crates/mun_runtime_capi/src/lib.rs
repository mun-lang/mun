//! The Mun Runtime C API
//!
//! The Mun Runtime C API exposes runtime functionality using the C ABI. This can be used to
//! integrate the Mun Runtime into other languages that allow interoperability with C.
#![warn(missing_docs)]

pub mod error;
pub mod hub;

#[cfg(test)]
mod tests;

use std::ffi::{c_void, CStr, CString};
use std::{os::raw::c_char, time::Duration};

use crate::error::ErrorHandle;
use crate::hub::HUB;
use failure::err_msg;
use mun_abi::{FunctionInfo, StructInfo, TypeInfo};
use mun_runtime::{Runtime, RuntimeOptions};

pub(crate) type Token = usize;

/// A type to uniquely index typed collections.
pub trait TypedHandle {
    /// Constructs a new `TypedHandle`.
    fn new(token: Token) -> Self;
    /// Retrieves the handle's token.
    fn token(&self) -> Token;
}

/// A C-style handle to a runtime.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RuntimeHandle(*mut c_void);

/// Constructs a new runtime that loads the library at `library_path` and its dependencies. If
/// successful, the runtime `handle` is set, otherwise a non-zero error handle is returned.
///
/// If a non-zero error handle is returned, it must be manually destructed using
/// [`mun_error_destroy`].
///
/// The runtime must be manually destructed using [`mun_runtime_destroy`].
///
/// TODO: expose interval at which the runtime's file watcher operates.
///
/// # Safety
///
/// This function receives raw pointers as parameters. If any of the arguments is a null pointer,
/// an error will be returned. Passing pointers to invalid data, will lead to undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn mun_runtime_create(
    library_path: *const c_char,
    handle: *mut RuntimeHandle,
) -> ErrorHandle {
    if library_path.is_null() {
        return HUB.errors.register(Box::new(err_msg(
            "Invalid argument: 'library_path' is null pointer.",
        )));
    }

    let library_path = match CStr::from_ptr(library_path).to_str() {
        Ok(path) => path,
        Err(_) => {
            return HUB.errors.register(Box::new(err_msg(
                "Invalid argument: 'library_path' is not UTF-8 encoded.",
            )))
        }
    };

    let handle = match handle.as_mut() {
        Some(handle) => handle,
        None => {
            return HUB.errors.register(Box::new(err_msg(
                "Invalid argument: 'handle' is null pointer.",
            )))
        }
    };

    let options = RuntimeOptions {
        library_path: library_path.into(),
        delay: Duration::from_millis(10),
        user_functions: Default::default(),
    };

    let runtime = match Runtime::new(options) {
        Ok(runtime) => runtime,
        Err(e) => return HUB.errors.register(Box::new(e)),
    };

    handle.0 = Box::into_raw(Box::new(runtime)) as *mut _;
    ErrorHandle::default()
}

/// Destructs the runtime corresponding to `handle`.
#[no_mangle]
pub extern "C" fn mun_runtime_destroy(handle: RuntimeHandle) {
    if !handle.0.is_null() {
        let _runtime = unsafe { Box::from_raw(handle.0) };
    }
}

/// Retrieves the [`FunctionInfo`] for `fn_name` from the runtime corresponding to `handle`. If
/// successful, `has_fn_info` and `fn_info` are set, otherwise a non-zero error handle is returned.
///
/// If a non-zero error handle is returned, it must be manually destructed using
/// [`mun_error_destroy`].
///
/// # Safety
///
/// This function receives raw pointers as parameters. If any of the arguments is a null pointer,
/// an error will be returned. Passing pointers to invalid data, will lead to undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn mun_runtime_get_function_info(
    handle: RuntimeHandle,
    fn_name: *const c_char,
    has_fn_info: *mut bool,
    fn_info: *mut FunctionInfo,
) -> ErrorHandle {
    let runtime = match (handle.0 as *mut Runtime).as_ref() {
        Some(runtime) => runtime,
        None => {
            return HUB.errors.register(Box::new(err_msg(
                "Invalid argument: 'runtime' is null pointer.",
            )))
        }
    };

    if fn_name.is_null() {
        return HUB.errors.register(Box::new(err_msg(
            "Invalid argument: 'fn_name' is null pointer.",
        )));
    }

    let fn_name = match CStr::from_ptr(fn_name).to_str() {
        Ok(name) => name,
        Err(_) => {
            return HUB.errors.register(Box::new(err_msg(
                "Invalid argument: 'fn_name' is not UTF-8 encoded.",
            )))
        }
    };

    let has_fn_info = match has_fn_info.as_mut() {
        Some(has_info) => has_info,
        None => {
            return HUB.errors.register(Box::new(err_msg(
                "Invalid argument: 'has_fn_info' is null pointer.",
            )))
        }
    };

    let fn_info = match fn_info.as_mut() {
        Some(info) => info,
        None => {
            return HUB.errors.register(Box::new(err_msg(
                "Invalid argument: 'fn_info' is null pointer.",
            )))
        }
    };

    match runtime.get_function_info(fn_name) {
        Some(info) => {
            *has_fn_info = true;
            *fn_info = info.clone();
        }
        None => *has_fn_info = false,
    }

    ErrorHandle::default()
}

/// Updates the runtime corresponding to `handle`. If successful, `updated` is set, otherwise a
/// non-zero error handle is returned.
///
/// If a non-zero error handle is returned, it must be manually destructed using
/// [`mun_error_destroy`].
///
/// # Safety
///
/// This function receives raw pointers as parameters. If any of the arguments is a null pointer,
/// an error will be returned. Passing pointers to invalid data, will lead to undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn mun_runtime_update(
    handle: RuntimeHandle,
    updated: *mut bool,
) -> ErrorHandle {
    let runtime = match (handle.0 as *mut Runtime).as_mut() {
        Some(runtime) => runtime,
        None => {
            return HUB.errors.register(Box::new(err_msg(
                "Invalid argument: 'runtime' is null pointer.",
            )))
        }
    };

    let updated = match updated.as_mut() {
        Some(updated) => updated,
        None => {
            return HUB.errors.register(Box::new(err_msg(
                "Invalid argument: 'updated' is null pointer.",
            )))
        }
    };

    *updated = runtime.update();
    ErrorHandle::default()
}

/// Retrieves the [`StructInfo`] corresponding to `type_info`, if the type is a struct. If
/// successful, `struct_info` is set, otherwise a non-zero error handle is returned.
///
/// If a non-zero error handle is returned, it must be manually destructed using
/// [`mun_error_destroy`].
///
/// # Safety
///
/// This function receives raw pointers as parameters. If any of the arguments is a null pointer,
/// an error will be returned. Passing pointers to invalid data, will lead to undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn mun_type_info_as_struct(
    type_info: *const TypeInfo,
    struct_info: *mut StructInfo,
) -> ErrorHandle {
    let type_info = match type_info.as_ref() {
        Some(info) => info,
        None => {
            return HUB.errors.register(Box::new(err_msg(
                "Invalid argument: 'type_info' is null pointer.",
            )))
        }
    };

    let struct_info = match struct_info.as_mut() {
        Some(info) => info,
        None => {
            return HUB.errors.register(Box::new(err_msg(
                "Invalid argument: 'struct_info' is null pointer.",
            )))
        }
    };

    match type_info.as_struct() {
        Some(info) => *struct_info = info.clone(),
        None => {
            return HUB.errors.register(Box::new(err_msg(format!(
                "`{}` is not a struct.",
                type_info.name()
            ))))
        }
    }

    ErrorHandle::default()
}

/// Deallocates a string that was allocated by the runtime.
///
/// # Safety
///
/// This function receives a raw pointer as parameter. Only when the argument is not a null pointer,
/// its content will be deallocated. Passing pointers to invalid data or memory allocated by other
/// processes, will lead to undefined behavior.
#[no_mangle]
pub unsafe fn mun_destroy_string(string: *const c_char) {
    if !string.is_null() {
        // Destroy the string
        let _string = CString::from_raw(string as *mut _);
    }
}

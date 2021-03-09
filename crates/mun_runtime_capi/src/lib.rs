//! The Mun Runtime C API
//!
//! The Mun Runtime C API exposes runtime functionality using the C ABI. This can be used to
//! integrate the Mun Runtime into other languages that allow interoperability with C.
#![warn(missing_docs)]

pub mod error;
pub mod gc;
pub mod hub;

#[cfg(test)]
mod tests;

use std::{
    ffi::{c_void, CStr, CString},
    os::raw::c_char,
};

use crate::{error::ErrorHandle, hub::HUB};
use anyhow::anyhow;
use runtime::Runtime;

pub(crate) type Token = usize;

pub use memory::gc::GcPtr;

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

/// Options required to construct a [`RuntimeHandle`] through [`mun_runtime_create`]
///
/// # Safety
///
/// This struct contains raw pointers as parameters. Passing pointers to invalid data, will lead to
/// undefined behavior.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RuntimeOptions {
    /// Function definitions that should be inserted in the runtime before a mun library is loaded.
    /// This is useful to initialize `extern` functions used in a mun library.
    ///
    /// If the [`num_functions`] fields is non-zero this field must contain a pointer to an array
    /// of [`abi::FunctionDefinition`]s.
    pub functions: *const abi::FunctionDefinition,

    /// The number of functions in the [`functions`] array.
    pub num_functions: u32,
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        RuntimeOptions {
            functions: std::ptr::null(),
            num_functions: 0,
        }
    }
}

/// Constructs a new runtime that loads the library at `library_path` and its dependencies. If
/// successful, the runtime `handle` is set, otherwise a non-zero error handle is returned.
///
/// If a non-zero error handle is returned, it must be manually destructed using
/// [`mun_error_destroy`].
///
/// The runtime must be manually destructed using [`mun_runtime_destroy`].
///
/// # Safety
///
/// This function receives raw pointers as parameters. If any of the arguments is a null pointer,
/// an error will be returned. Passing pointers to invalid data, will lead to undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn mun_runtime_create(
    library_path: *const c_char,
    options: RuntimeOptions,
    handle: *mut RuntimeHandle,
) -> ErrorHandle {
    if library_path.is_null() {
        return HUB
            .errors
            .register(anyhow!("Invalid argument: 'library_path' is null pointer."));
    }

    if options.num_functions > 0 && options.functions.is_null() {
        return HUB
            .errors
            .register(anyhow!("Invalid argument: 'functions' is null pointer."));
    }

    let library_path = match CStr::from_ptr(library_path).to_str() {
        Ok(path) => path,
        Err(_) => {
            return HUB.errors.register(anyhow!(
                "Invalid argument: 'library_path' is not UTF-8 encoded.",
            ))
        }
    };

    let handle = match handle.as_mut() {
        Some(handle) => handle,
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'handle' is null pointer."))
        }
    };

    let user_functions =
        std::slice::from_raw_parts(options.functions, options.num_functions as usize)
            .iter()
            .map(|def| {
                abi::FunctionDefinitionStorage::new_function(
                    def.prototype.name(),
                    def.prototype.signature.arg_types(),
                    def.prototype.signature.return_type(),
                    def.fn_ptr,
                )
            })
            .collect();

    let runtime_options = runtime::RuntimeOptions {
        library_path: library_path.into(),
        user_functions,
    };

    let runtime = match Runtime::new(runtime_options) {
        Ok(runtime) => runtime,
        Err(e) => return HUB.errors.register(e),
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

/// Retrieves the [`FunctionDefinition`] for `fn_name` from the runtime corresponding to `handle`.
/// If successful, `has_fn_info` and `fn_info` are set, otherwise a non-zero error handle is
/// returned.
///
/// If a non-zero error handle is returned, it must be manually destructed using
/// [`mun_error_destroy`].
///
/// # Safety
///
/// This function receives raw pointers as parameters. If any of the arguments is a null pointer,
/// an error will be returned. Passing pointers to invalid data, will lead to undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn mun_runtime_get_function_definition(
    handle: RuntimeHandle,
    fn_name: *const c_char,
    has_fn_info: *mut bool,
    fn_definition: *mut abi::FunctionDefinition,
) -> ErrorHandle {
    let runtime = match (handle.0 as *mut Runtime).as_ref() {
        Some(runtime) => runtime,
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'runtime' is null pointer."))
        }
    };

    if fn_name.is_null() {
        return HUB
            .errors
            .register(anyhow!("Invalid argument: 'fn_name' is null pointer."));
    }

    let fn_name = match CStr::from_ptr(fn_name).to_str() {
        Ok(name) => name,
        Err(_) => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'fn_name' is not UTF-8 encoded."))
        }
    };

    let has_fn_info = match has_fn_info.as_mut() {
        Some(has_info) => has_info,
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'has_fn_info' is null pointer."))
        }
    };

    let fn_definition = match fn_definition.as_mut() {
        Some(info) => info,
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'fn_info' is null pointer."))
        }
    };

    match runtime.get_function_definition(fn_name) {
        Some(info) => {
            *has_fn_info = true;
            *fn_definition = info.clone();
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
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'runtime' is null pointer."))
        }
    };

    let updated = match updated.as_mut() {
        Some(updated) => updated,
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'updated' is null pointer."))
        }
    };

    *updated = runtime.update();
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
pub unsafe extern "C" fn mun_destroy_string(string: *const c_char) {
    if !string.is_null() {
        // Destroy the string
        let _string = CString::from_raw(string as *mut _);
    }
}

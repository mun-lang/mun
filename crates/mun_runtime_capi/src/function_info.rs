use crate::{error::ErrorHandle, hub::HUB, type_info::TypeInfoHandle};
use anyhow::anyhow;
use runtime::FunctionDefinition;
use std::{
    ffi::{c_void, CString},
    os::raw::c_char,
    ptr,
    sync::Arc,
};

/// A C-style handle to a `FunctionInfo`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FunctionInfoHandle(pub *const c_void);

impl FunctionInfoHandle {
    /// A null handle.
    pub fn null() -> Self {
        Self(ptr::null())
    }
}

/// Decrements the strong count of the `Arc<FunctionDefinition>` associated with `handle`.
#[no_mangle]
pub unsafe extern "C" fn mun_function_info_decrement_strong_count(fn_info: FunctionInfoHandle) {
    if !fn_info.0.is_null() {
        Arc::decrement_strong_count(fn_info.0);
    }
}

/// Increments the strong count of the `Arc<FunctionDefinition>` associated with `handle`.
#[no_mangle]
pub unsafe extern "C" fn mun_function_info_increment_strong_count(fn_info: FunctionInfoHandle) {
    if !fn_info.0.is_null() {
        Arc::increment_strong_count(fn_info.0);
    }
}

/// Retrieves the function's function pointer.
#[no_mangle]
pub unsafe extern "C" fn mun_function_info_fn_ptr(fn_info: FunctionInfoHandle) -> *const c_void {
    let fn_def = match (fn_info.0 as *const FunctionDefinition).as_ref() {
        Some(fn_def) => fn_def,
        None => return ptr::null(),
    };

    fn_def.fn_ptr
}

/// Retrieves the function's name.
///
/// # Safety
///
/// The caller is responsible for calling `mun_destroy_string` on the return pointer - if it is not null.
#[no_mangle]
pub unsafe extern "C" fn mun_function_info_name(fn_info: FunctionInfoHandle) -> *const c_char {
    let fn_def = match (fn_info.0 as *const FunctionDefinition).as_ref() {
        Some(fn_def) => fn_def,
        None => return ptr::null(),
    };

    CString::new(fn_def.prototype.name.clone())
        .unwrap()
        .into_raw() as *const _
}

/// Retrieves the function's argument types.
#[no_mangle]
pub unsafe extern "C" fn mun_function_info_argument_types(
    fn_info: FunctionInfoHandle,
    arg_types_begin: *mut TypeInfoHandle,
    num_args: *mut usize,
) -> ErrorHandle {
    let fn_info = match (fn_info.0 as *const FunctionDefinition).as_ref() {
        Some(fn_info) => fn_info,
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'fn_info' is null pointer."))
        }
    };

    let arg_types_begin = match arg_types_begin.as_mut() {
        Some(arg_types_begin) => arg_types_begin,
        None => {
            return HUB.errors.register(anyhow!(
                "Invalid argument: 'arg_types_begin' is null pointer."
            ))
        }
    };

    let num_args = match num_args.as_mut() {
        Some(num_args) => num_args,
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'num_args' is null pointer."))
        }
    };

    let fn_sig = &fn_info.prototype.signature;
    // TODO: Clone Arc<TypeInfo>
    arg_types_begin.0 = fn_sig.arg_types.as_ptr() as *const c_void;
    *num_args = fn_sig.arg_types.len();

    ErrorHandle::default()
}

/// Retrieves the function's return type.
#[no_mangle]
pub unsafe extern "C" fn mun_function_info_return_type(
    fn_info: FunctionInfoHandle,
) -> TypeInfoHandle {
    let fn_info = match (fn_info.0 as *const FunctionDefinition).as_ref() {
        Some(fn_info) => fn_info,
        None => return TypeInfoHandle::null(),
    };

    TypeInfoHandle(Arc::into_raw(fn_info.prototype.signature.return_type.clone()) as *const c_void)
}

// TODO: Add tests

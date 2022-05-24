use std::{ffi::c_void, ptr, sync::Arc};

/// A C-style handle to a `FunctionInfo`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FunctionInfoHandle(pub *const c_void);

impl FunctionInfoHandle {
    pub fn null() -> Self {
        Self(ptr::null())
    }
}

/// Decrements the strong count of the `Arc<FunctionDefinition>` associated with `handle`.
#[no_mangle]
pub unsafe extern "C" fn mun_function_info_decrement_strong_count(fn_def: FunctionInfoHandle) {
    if !fn_def.0.is_null() {
        Arc::decrement_strong_count(fn_def.0);
    }
}

/// Increments the strong count of the `Arc<FunctionDefinition>` associated with `handle`.
#[no_mangle]
pub unsafe extern "C" fn mun_function_info_increment_strong_count(fn_def: FunctionInfoHandle) {
    if !fn_def.0.is_null() {
        Arc::increment_strong_count(fn_def.0);
    }
}

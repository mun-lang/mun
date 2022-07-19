//! Exposes function information using the C ABI.

use capi_utils::error::ErrorHandle;
use capi_utils::{mun_error_try, try_deref_mut};
use memory::ffi::{Type, Types};
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
///
/// # Safety
///
/// This function might be unsafe if the underlying data has already been deallocated by a previous
/// call to [`mun_function_info_decrement_strong_count`].
#[no_mangle]
pub unsafe extern "C" fn mun_function_info_decrement_strong_count(
    fn_info: FunctionInfoHandle,
) -> bool {
    if !fn_info.0.is_null() {
        Arc::decrement_strong_count(fn_info.0);
        return true;
    }

    false
}

/// Increments the strong count of the `Arc<FunctionDefinition>` associated with `handle`.
///
/// # Safety
///
/// This function might be unsafe if the underlying data has been deallocated by a previous call
/// to [`mun_function_info_decrement_strong_count`].
#[no_mangle]
pub unsafe extern "C" fn mun_function_info_increment_strong_count(
    fn_info: FunctionInfoHandle,
) -> bool {
    if !fn_info.0.is_null() {
        Arc::increment_strong_count(fn_info.0);
        return true;
    }

    false
}

/// Retrieves the function's function pointer.
///
/// # Safety
///
/// This function might be unsafe if the underlying data has been deallocated by a previous call
/// to [`mun_function_info_decrement_strong_count`].
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
/// The caller is responsible for calling `mun_string_destroy` on the return pointer - if it is not null.
///
/// This function might be unsafe if the underlying data has been deallocated by a previous call
/// to [`mun_function_info_decrement_strong_count`].
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
///
/// # Safety
///
/// If a non-null handle is returned, the caller is responsible for calling
/// `mun_type_info_span_destroy` on the returned handle.
///
/// This function might be unsafe if the underlying data has been deallocated by a previous call
/// to [`mun_function_info_decrement_strong_count`].
#[no_mangle]
pub unsafe extern "C" fn mun_function_info_argument_types(
    fn_info: FunctionInfoHandle,
    arg_types: *mut Types,
) -> ErrorHandle {
    let fn_info = mun_error_try!((fn_info.0 as *const FunctionDefinition)
        .as_ref()
        .ok_or("FunctionInfoHandle contains invalid pointer"));
    let arg_types = try_deref_mut!(arg_types);
    *arg_types = fn_info
        .prototype
        .signature
        .arg_types
        .iter()
        .map(|ty| ty.clone().into())
        .collect::<Vec<_>>()
        .into();
    ErrorHandle::default()
}

/// Retrieves the function's return type.
///
/// # Safety
///
/// This function might be unsafe if the underlying data has been deallocated by a previous call
/// to [`mun_function_info_decrement_strong_count`].
#[no_mangle]
pub unsafe extern "C" fn mun_function_info_return_type(
    fn_info: FunctionInfoHandle,
    ty: *mut Type,
) -> ErrorHandle {
    let fn_info = mun_error_try!((fn_info.0 as *const FunctionDefinition)
        .as_ref()
        .ok_or("FunctionInfoHandle contains invalid pointer"));
    let ty = try_deref_mut!(ty);
    *ty = fn_info.prototype.signature.return_type.clone().into();
    ErrorHandle::default()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        runtime::{mun_runtime_get_function_info, RuntimeHandle},
        test_util::TestDriver,
    };
    use capi_utils::error::mun_error_destroy;
    use capi_utils::{assert_error, mun_string_destroy};
    use memory::ffi::mun_type_equal;
    use memory::HasStaticType;
    use runtime::FunctionDefinition;
    use std::{
        ffi::{CStr, CString},
        mem::{self, MaybeUninit},
        slice,
        sync::Arc,
    };

    pub(crate) fn get_fake_function_info<T: Into<Vec<u8>>>(
        runtime: RuntimeHandle,
        fn_name: T,
    ) -> FunctionInfoHandle {
        let fn_name = CString::new(fn_name).expect("Invalid function name");
        let mut has_fn_info = false;
        let mut fn_definition = MaybeUninit::uninit();
        let handle = unsafe {
            mun_runtime_get_function_info(
                runtime,
                fn_name.as_ptr(),
                &mut has_fn_info as *mut _,
                fn_definition.as_mut_ptr(),
            )
        };
        assert_eq!(handle.0, ptr::null());
        assert!(has_fn_info);

        unsafe { fn_definition.assume_init() }
    }

    #[test]
    fn test_function_info_decrement_strong_count_invalid_fn_info() {
        assert!(!unsafe { mun_function_info_decrement_strong_count(FunctionInfoHandle::null()) },);
    }

    #[test]
    fn test_function_info_decrement_strong_count() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 12345 }
    "#,
        );

        let fn_info = get_fake_function_info(driver.runtime, "main");

        let fn_info_arc = unsafe { Arc::from_raw(fn_info.0 as *const FunctionDefinition) };
        let strong_count = Arc::strong_count(&fn_info_arc);
        assert!(strong_count > 0);

        assert!(unsafe { mun_function_info_decrement_strong_count(fn_info) });
        assert_eq!(Arc::strong_count(&fn_info_arc), strong_count - 1);

        mem::forget(fn_info_arc);
    }

    #[test]
    fn test_function_info_increment_strong_count_invalid_fn_info() {
        assert!(!unsafe { mun_function_info_increment_strong_count(FunctionInfoHandle::null()) });
    }

    #[test]
    fn test_function_info_increment_strong_count() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 12345 }
    "#,
        );

        let fn_info = get_fake_function_info(driver.runtime, "main");

        let fn_info_arc = unsafe { Arc::from_raw(fn_info.0 as *const FunctionDefinition) };
        let strong_count = Arc::strong_count(&fn_info_arc);
        assert!(strong_count > 0);

        assert!(unsafe { mun_function_info_increment_strong_count(fn_info) });
        assert_eq!(Arc::strong_count(&fn_info_arc), strong_count + 1);

        mem::forget(fn_info_arc);
    }

    #[test]
    fn test_function_info_fn_ptr_invalid_fn_info() {
        let fn_ptr = unsafe { mun_function_info_fn_ptr(FunctionInfoHandle::null()) };
        assert_eq!(fn_ptr, ptr::null());
    }

    #[test]
    fn test_function_info_fn_ptr() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 12345 }
    "#,
        );

        let fn_info = get_fake_function_info(driver.runtime, "main");
        let fn_ptr = unsafe { mun_function_info_fn_ptr(fn_info) };
        assert_ne!(fn_ptr, ptr::null());
    }

    #[test]
    fn test_function_info_name_invalid_fn_info() {
        let fn_ptr = unsafe { mun_function_info_name(FunctionInfoHandle::null()) };
        assert_eq!(fn_ptr, ptr::null());
    }

    #[test]
    fn test_function_info_name() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 12345 }
    "#,
        );

        let fn_info = get_fake_function_info(driver.runtime, "main");
        let name = unsafe { mun_function_info_name(fn_info) };
        assert_ne!(name, ptr::null());

        let name_str = unsafe { CStr::from_ptr(name) }
            .to_str()
            .expect("Invalid function name.");

        assert_eq!(name_str, "main");

        unsafe { mun_string_destroy(name) };
    }

    #[test]
    fn test_function_info_argument_types_invalid_fn_info() {
        let handle = unsafe {
            mun_function_info_argument_types(FunctionInfoHandle::null(), ptr::null_mut())
        };
        assert_ne!(handle.0, ptr::null());

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'fn_info' is null pointer."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_function_info_argument_types_invalid_arg_types() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 12345 }
    "#,
        );

        let fn_info = get_fake_function_info(driver.runtime, "main");
        let handle = unsafe { mun_function_info_argument_types(fn_info, ptr::null_mut()) };
        assert_ne!(handle.0, ptr::null());

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'arg_types' is null pointer."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_function_info_argument_types_none() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 12345 }
    "#,
        );

        let fn_info = get_fake_function_info(driver.runtime, "main");
        let mut arg_types = MaybeUninit::uninit();
        assert!(
            unsafe { mun_function_info_argument_types(fn_info, arg_types.as_mut_ptr()) }.is_ok()
        );
        let arg_types = unsafe { arg_types.assume_init() };

        assert_eq!(arg_types.types, ptr::null());
        assert_eq!(arg_types.count, 0);
    }

    #[test]
    fn test_function_info_argument_types_some() {
        let driver = TestDriver::new(
            r#"
        pub fn add(a: i32, b: i32) -> i32 { a + b }
    "#,
        );

        let fn_info = get_fake_function_info(driver.runtime, "add");
        let mut arg_types = MaybeUninit::uninit();
        assert!(
            unsafe { mun_function_info_argument_types(fn_info, arg_types.as_mut_ptr()) }.is_ok()
        );

        let arg_types = unsafe { arg_types.assume_init() };
        assert_eq!(arg_types.count, 2);

        let arg_types = unsafe { slice::from_raw_parts(arg_types.types, arg_types.count) };

        for arg_type in arg_types {
            assert!(unsafe { mun_type_equal(*arg_type, i32::type_info().clone().into()) });
        }
    }

    #[test]
    fn test_function_info_return_type_invalid_fn_info() {
        assert_error!(unsafe {
            mun_function_info_return_type(FunctionInfoHandle::null(), ptr::null_mut())
        });
    }

    #[test]
    fn test_function_info_return_type_none() {
        let driver = TestDriver::new(
            r#"
        pub fn main() { }
    "#,
        );

        let fn_info = get_fake_function_info(driver.runtime, "main");

        let mut return_type = MaybeUninit::uninit();
        assert!(
            unsafe { mun_function_info_return_type(fn_info, return_type.as_mut_ptr()) }.is_ok()
        );
        let return_type = unsafe { return_type.assume_init() };

        assert!(unsafe { mun_type_equal(return_type, <()>::type_info().clone().into()) });
    }

    #[test]
    fn test_function_info_return_type_some() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 12345 }
    "#,
        );

        let fn_info = get_fake_function_info(driver.runtime, "main");
        let mut return_type = MaybeUninit::uninit();
        assert!(
            unsafe { mun_function_info_return_type(fn_info, return_type.as_mut_ptr()) }.is_ok()
        );
        let return_type = unsafe { return_type.assume_init() };

        assert!(unsafe { mun_type_equal(return_type, <i32>::type_info().clone().into()) });
    }
}

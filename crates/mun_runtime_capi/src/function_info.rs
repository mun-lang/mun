//! Exposes function information using the C ABI.

use crate::{
    error::ErrorHandle,
    type_info::{TypeInfoHandle, TypeInfoSpan},
};
use memory::TypeInfo;
use runtime::FunctionDefinition;
use std::{
    ffi::{c_void, CString},
    mem,
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
    arg_types: *mut TypeInfoSpan,
) -> ErrorHandle {
    let fn_info = match (fn_info.0 as *const FunctionDefinition).as_ref() {
        Some(fn_info) => fn_info,
        None => return ErrorHandle::new("Invalid argument: 'fn_info' is null pointer."),
    };

    let arg_types = match arg_types.as_mut() {
        Some(arg_types) => arg_types,
        None => return ErrorHandle::new("Invalid argument: 'arg_types' is null pointer."),
    };

    let fn_sig = &fn_info.prototype.signature;
    let mut types: Vec<*const TypeInfo> = fn_sig
        .arg_types
        .iter()
        .map(|ty| Arc::into_raw(ty.clone()))
        .collect();

    arg_types.data = if !types.is_empty() {
        types.shrink_to_fit();
        types.as_ptr() as *const _
    } else {
        ptr::null()
    };
    arg_types.len = types.len();

    // Move ownership to caller
    mem::forget(types);

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
) -> TypeInfoHandle {
    let fn_info = match (fn_info.0 as *const FunctionDefinition).as_ref() {
        Some(fn_info) => fn_info,
        None => return TypeInfoHandle::null(),
    };

    TypeInfoHandle(Arc::into_raw(fn_info.prototype.signature.return_type.clone()) as *const c_void)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        error::mun_error_destroy,
        mun_string_destroy,
        runtime::{mun_runtime_get_function_info, RuntimeHandle},
        test_util::TestDriver,
        type_info::mun_type_info_eq,
    };
    use memory::HasStaticTypeInfo;
    use runtime::FunctionDefinition;
    use std::{
        ffi::{CStr, CString},
        mem::{self, MaybeUninit},
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
        let handle = unsafe { mun_function_info_argument_types(fn_info, arg_types.as_mut_ptr()) };
        assert_eq!(handle.0, ptr::null());

        let arg_types = unsafe { arg_types.assume_init() };
        assert_eq!(arg_types.data, ptr::null());
        assert_eq!(arg_types.len, 0);
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
        let handle = unsafe { mun_function_info_argument_types(fn_info, arg_types.as_mut_ptr()) };
        assert_eq!(handle.0, ptr::null());

        let arg_types = unsafe { arg_types.assume_init() };
        assert_eq!(arg_types.len, 2);

        (0..arg_types.len).into_iter().for_each(|index| {
            let type_info = arg_types.get(index);
            assert_ne!(type_info.0, ptr::null());
            assert!(unsafe { mun_type_info_eq(type_info, i32::type_info().clone().into()) });
        })
    }

    #[test]
    fn test_function_info_return_type_invalid_fn_info() {
        let return_type = unsafe { mun_function_info_return_type(FunctionInfoHandle::null()) };
        assert_eq!(return_type.0, ptr::null());
    }

    #[test]
    fn test_function_info_return_type_none() {
        let driver = TestDriver::new(
            r#"
        pub fn main() { }
    "#,
        );

        let fn_info = get_fake_function_info(driver.runtime, "main");
        let return_type = unsafe { mun_function_info_return_type(fn_info) };
        assert_ne!(return_type.0, ptr::null());

        assert!(unsafe { mun_type_info_eq(return_type, <()>::type_info().clone().into()) });
    }

    #[test]
    fn test_function_info_return_type_some() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 12345 }
    "#,
        );

        let fn_info = get_fake_function_info(driver.runtime, "main");
        let return_type = unsafe { mun_function_info_return_type(fn_info) };
        assert_ne!(return_type.0, ptr::null());

        assert!(unsafe { mun_type_info_eq(return_type, <i32>::type_info().clone().into()) });
    }
}

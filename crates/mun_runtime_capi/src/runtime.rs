//! Exposes the Mun runtime using the C ABI.

use capi_utils::error::ErrorHandle;
use capi_utils::{mun_error_try, try_convert_c_string, try_deref, try_deref_mut};
use memory::ffi::Type;
use memory::type_table::TypeTable;
use memory::Type as RustType;
use runtime::{FunctionDefinition, FunctionPrototype, FunctionSignature, Runtime};
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::{
    ffi::{c_void, CStr},
    os::raw::c_char,
    sync::Arc,
};

use crate::function_info::FunctionInfoHandle;

/// A C-style handle to a runtime.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RuntimeHandle(pub *mut c_void);

impl RuntimeHandle {
    pub unsafe fn inner(&self) -> Result<&Runtime, String> {
        (self.0 as *mut Runtime)
            .as_ref()
            .ok_or_else(|| String::from("RuntimeHandle is null pointer."))
    }

    pub unsafe fn inner_mut(&self) -> Result<&mut Runtime, String> {
        (self.0 as *mut Runtime)
            .as_mut()
            .ok_or_else(|| String::from("RuntimeHandle is null pointer."))
    }
}

/// Definition of an external function that is callable from Mun.
///
/// The ownership of the contained TypeInfoHandles is considered to lie with this struct.
#[repr(C)]
#[derive(Clone)]
pub struct ExternalFunctionDefinition {
    /// The name of the function
    pub name: *const c_char,

    /// The number of arguments of the function
    pub num_args: u32,

    /// The types of the arguments
    pub arg_types: *const Type,

    /// The type of the return type
    pub return_type: Type,

    /// Pointer to the function
    pub fn_ptr: *const c_void,
}

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
    pub functions: *const ExternalFunctionDefinition,

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
        return ErrorHandle::new("invalid argument: 'library_path' is null pointer.");
    }

    if options.num_functions > 0 && options.functions.is_null() {
        return ErrorHandle::new("invalid argument: 'functions' is null pointer.");
    }

    let library_path = match CStr::from_ptr(library_path).to_str() {
        Ok(path) => path,
        Err(_) => {
            return ErrorHandle::new("invalid argument: 'library_path' is not UTF-8 encoded.");
        }
    };

    let handle = match handle.as_mut() {
        Some(handle) => handle,
        None => return ErrorHandle::new("invalid argument: 'handle' is null pointer."),
    };

    let type_table = TypeTable::default();
    let user_functions = mun_error_try!(std::slice::from_raw_parts(
        options.functions,
        options.num_functions as usize
    )
    .iter()
    .map(|def| {
        let name =
            try_convert_c_string(def.name).map_err(|e| format!("invalid function name: {e}"))?;
        let return_type = ManuallyDrop::new(
            def.return_type
                .to_owned()
                .map_err(|e| format!("invalid function '{name}': 'return_type': {e}"))?,
        )
        .deref()
        .clone();

        if def.num_args > 0 && def.arg_types.is_null() {
            return Err(format!(
                "invalid function '{}': 'arg_types' is null pointer.",
                name
            ));
        }

        let arg_types: Vec<_> = if def.num_args > 0 {
            std::slice::from_raw_parts(def.arg_types, def.num_args as usize)
                .iter()
                .enumerate()
                .map(|(i, arg)| -> Result<RustType, String> {
                    let ty = (*arg).to_owned().map_err(|e| {
                        format!("invalid function '{}': argument #{}: {}", name, i + 1, e)
                    })?;
                    Ok(ManuallyDrop::new(ty.to_owned()).deref().clone())
                })
                .collect::<Result<_, _>>()?
        } else {
            Vec::new()
        };

        Ok(FunctionDefinition {
            prototype: FunctionPrototype {
                name: name.to_owned(),
                signature: FunctionSignature {
                    arg_types,
                    return_type,
                },
            },
            fn_ptr: def.fn_ptr,
        })
    })
    .collect::<Result<_, _>>());

    let runtime_options = runtime::RuntimeOptions {
        library_path: library_path.into(),
        user_functions,
        type_table,
    };

    let runtime = match Runtime::new(runtime_options) {
        Ok(runtime) => runtime,
        Err(e) => return ErrorHandle::new(e.to_string()),
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

/// Retrieves the [`FunctionDefinition`] for `fn_name` from the `runtime`. If successful,
/// `has_fn_info` and `fn_info` are set, otherwise a non-zero error handle is returned.
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
    runtime: RuntimeHandle,
    fn_name: *const c_char,
    has_fn_info: *mut bool,
    fn_info: *mut FunctionInfoHandle,
) -> ErrorHandle {
    let runtime = mun_error_try!(runtime.inner());
    let name = mun_error_try!(
        try_convert_c_string(fn_name).map_err(|e| format!("invalid 'fn_name': {e}"))
    );
    let has_fn_info = try_deref_mut!(has_fn_info);
    let fn_info = try_deref_mut!(fn_info);
    match runtime.get_function_definition(name) {
        Some(info) => {
            *has_fn_info = true;
            fn_info.0 = Arc::into_raw(info) as *const c_void;
        }
        None => *has_fn_info = false,
    }

    ErrorHandle::default()
}

/// Retrieves the type information corresponding to the specified `type_name` from the runtime.
/// If successful, `has_type_info` and `type_info` are set, otherwise a non-zero error handle is
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
pub unsafe extern "C" fn mun_runtime_get_type_info_by_name(
    runtime: RuntimeHandle,
    type_name: *const c_char,
    has_type_info: *mut bool,
    type_info: *mut Type,
) -> ErrorHandle {
    let runtime = mun_error_try!(runtime.inner());
    let type_name = mun_error_try!(
        try_convert_c_string(type_name).map_err(|e| format!("invalid 'type_name': {e}"))
    );
    let has_type_info = try_deref_mut!(has_type_info);
    let type_info = try_deref_mut!(type_info);
    match runtime.get_type_info_by_name(type_name) {
        Some(info) => {
            *has_type_info = true;
            *type_info = info.into();
        }
        None => *has_type_info = false,
    }

    ErrorHandle::default()
}

/// Retrieves the type information corresponding to the specified `type_id` from the runtime. If
/// successful, `has_type_info` and `type_info` are set, otherwise a non-zero error handle is
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
pub unsafe extern "C" fn mun_runtime_get_type_info_by_id(
    runtime: RuntimeHandle,
    type_id: *const abi::TypeId,
    has_type_info: *mut bool,
    type_info: *mut Type,
) -> ErrorHandle {
    let runtime = mun_error_try!(runtime.inner());
    let type_id = try_deref!(type_id);
    let has_type_info = try_deref_mut!(has_type_info);
    let type_info = try_deref_mut!(type_info);

    match runtime.get_type_info_by_id(type_id) {
        Some(info) => {
            *has_type_info = true;
            *type_info = info.into();
        }
        None => *has_type_info = false,
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
    runtime: RuntimeHandle,
    updated: *mut bool,
) -> ErrorHandle {
    let runtime = mun_error_try!(runtime.inner_mut());
    let updated = try_deref_mut!(updated);
    *updated = runtime.update();
    ErrorHandle::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test_invalid_runtime, test_util::TestDriver};
    use capi_utils::error::mun_error_destroy;
    use memory::HasStaticType;
    use std::{ffi::CString, mem::MaybeUninit, ptr};

    test_invalid_runtime!(
        runtime_get_function_info(ptr::null(), ptr::null_mut(), ptr::null_mut()),
        runtime_get_type_info_by_name(ptr::null(), ptr::null_mut(), ptr::null_mut()),
        runtime_get_type_info_by_id(ptr::null(), ptr::null_mut(), ptr::null_mut()),
        runtime_update(ptr::null_mut())
    );

    #[test]
    fn test_runtime_create_invalid_lib_path() {
        let handle =
            unsafe { mun_runtime_create(ptr::null(), RuntimeOptions::default(), ptr::null_mut()) };
        assert_ne!(handle.0, ptr::null());

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'library_path' is null pointer."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_runtime_create_invalid_lib_path_encoding() {
        let invalid_encoding = ['�', '\0'];

        let handle = unsafe {
            mun_runtime_create(
                invalid_encoding.as_ptr() as *const _,
                RuntimeOptions::default(),
                ptr::null_mut(),
            )
        };
        assert_ne!(handle.0, ptr::null());

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'library_path' is not UTF-8 encoded."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_runtime_create_invalid_functions() {
        let lib_path = CString::new("some/path").expect("Invalid library path");

        let options = RuntimeOptions {
            num_functions: 1,
            ..Default::default()
        };

        let handle = unsafe { mun_runtime_create(lib_path.into_raw(), options, ptr::null_mut()) };
        assert_ne!(handle.0, ptr::null());

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'functions' is null pointer."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_runtime_create_invalid_handle() {
        let lib_path = CString::new("some/path").expect("Invalid library path");

        let handle = unsafe {
            mun_runtime_create(
                lib_path.into_raw(),
                RuntimeOptions::default(),
                ptr::null_mut(),
            )
        };
        assert_ne!(handle.0, ptr::null());

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'handle' is null pointer."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_runtime_create_invalid_user_function_name() {
        let lib_path = CString::new("some/path").expect("Invalid library path");

        let type_id = <()>::type_info().clone().into();
        let functions = vec![ExternalFunctionDefinition {
            name: ptr::null(),
            arg_types: ptr::null(),
            return_type: type_id,
            num_args: 0,
            fn_ptr: ptr::null(),
        }];

        let options = RuntimeOptions {
            functions: functions.as_ptr(),
            num_functions: 1,
            ..Default::default()
        };

        let mut runtime = MaybeUninit::uninit();
        let handle =
            unsafe { mun_runtime_create(lib_path.into_raw(), options, runtime.as_mut_ptr()) };
        assert_ne!(handle.0, ptr::null());

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            format!("Invalid function name: function name null pointer")
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_runtime_create_invalid_user_function_name_encoding() {
        let lib_path = CString::new("some/path").expect("Invalid library path");

        let invalid_encoding = ['�', '\0'];
        let type_id = <()>::type_info().clone().into();
        let functions = vec![ExternalFunctionDefinition {
            name: invalid_encoding.as_ptr() as *const _,
            arg_types: ptr::null(),
            return_type: type_id,
            num_args: 0,
            fn_ptr: ptr::null(),
        }];

        let options = RuntimeOptions {
            functions: functions.as_ptr(),
            num_functions: 1,
            ..Default::default()
        };

        let mut runtime = MaybeUninit::uninit();
        let handle =
            unsafe { mun_runtime_create(lib_path.into_raw(), options, runtime.as_mut_ptr()) };
        assert_ne!(handle.0, ptr::null());

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            format!("Invalid function name: function name is not UTF-8 encoded: invalid utf-8 sequence of 1 bytes from index 0")
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_runtime_create_invalid_user_function_return_type() {
        let lib_path = CString::new("some/path").expect("Invalid library path");
        let function_name = CString::new("foobar").unwrap();

        let functions = vec![ExternalFunctionDefinition {
            name: function_name.as_ptr(),
            arg_types: ptr::null(),
            return_type: Type::null(),
            num_args: 0,
            fn_ptr: ptr::null(),
        }];

        let options = RuntimeOptions {
            functions: functions.as_ptr(),
            num_functions: 1,
            ..Default::default()
        };

        let mut runtime = MaybeUninit::uninit();
        let handle =
            unsafe { mun_runtime_create(lib_path.into_raw(), options, runtime.as_mut_ptr()) };
        assert_ne!(handle.0, ptr::null());

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            format!("Invalid function 'foobar': 'return_type' is null pointer.")
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_runtime_create_invalid_user_function_arg_types_ptr() {
        let lib_path = CString::new("some/path").expect("Invalid library path");
        let function_name = CString::new("foobar").unwrap();

        let type_id = <()>::type_info().clone().into();
        let functions = vec![ExternalFunctionDefinition {
            name: function_name.as_ptr(),
            arg_types: ptr::null(),
            return_type: type_id,
            num_args: 1,
            fn_ptr: ptr::null(),
        }];

        let options = RuntimeOptions {
            functions: functions.as_ptr(),
            num_functions: 1,
            ..Default::default()
        };

        let mut runtime = MaybeUninit::uninit();
        let handle =
            unsafe { mun_runtime_create(lib_path.into_raw(), options, runtime.as_mut_ptr()) };
        assert_ne!(handle.0, ptr::null());

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            format!("Invalid function 'foobar': 'arg_types' is null pointer.")
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_runtime_create_invalid_user_function_arg_types() {
        let lib_path = CString::new("some/path").expect("Invalid library path");
        let function_name = CString::new("foobar").unwrap();
        let arg_types = [Type::null()];

        let type_id = <()>::type_info().clone().into();
        let functions = vec![ExternalFunctionDefinition {
            name: function_name.as_ptr(),
            arg_types: &arg_types as _,
            return_type: type_id,
            num_args: 1,
            fn_ptr: ptr::null(),
        }];

        let options = RuntimeOptions {
            functions: functions.as_ptr(),
            num_functions: 1,
            ..Default::default()
        };

        let mut runtime = MaybeUninit::uninit();
        let handle =
            unsafe { mun_runtime_create(lib_path.into_raw(), options, runtime.as_mut_ptr()) };
        assert_ne!(handle.0, ptr::null());

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            format!("Invalid function 'foobar': argument #1 is null pointer.")
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_runtime_get_function_info_invalid_fn_name() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 3 }
    "#,
        );

        let handle = unsafe {
            mun_runtime_get_function_info(
                driver.runtime,
                ptr::null(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'fn_name' is null pointer."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_runtime_get_function_info_invalid_fn_name_encoding() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 3 }
    "#,
        );

        let invalid_encoding = ['�', '\0'];
        let handle = unsafe {
            mun_runtime_get_function_info(
                driver.runtime,
                invalid_encoding.as_ptr() as *const _,
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'fn_name' is not UTF-8 encoded."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_runtime_get_function_info_invalid_has_fn_info() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 3 }
    "#,
        );

        let fn_name = CString::new("main").expect("Invalid function name");
        let handle = unsafe {
            mun_runtime_get_function_info(
                driver.runtime,
                fn_name.as_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'has_fn_info' is null pointer."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_runtime_get_function_info_invalid_fn_info() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 3 }
    "#,
        );

        let fn_name = CString::new("main").expect("Invalid function name");
        let mut has_fn_info = false;
        let handle = unsafe {
            mun_runtime_get_function_info(
                driver.runtime,
                fn_name.as_ptr(),
                &mut has_fn_info as *mut _,
                ptr::null_mut(),
            )
        };

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'fn_info' is null pointer."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_runtime_get_function_info_none() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 3 }
    "#,
        );

        let fn_name = CString::new("add").expect("Invalid function name");
        let mut has_fn_info = false;
        let mut fn_definition = MaybeUninit::uninit();
        let handle = unsafe {
            mun_runtime_get_function_info(
                driver.runtime,
                fn_name.as_ptr(),
                &mut has_fn_info as *mut _,
                fn_definition.as_mut_ptr(),
            )
        };
        assert_eq!(handle.0, ptr::null());
        assert!(!has_fn_info);
    }

    #[test]
    fn test_runtime_get_function_info_some() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 3 }
    "#,
        );

        let fn_name = CString::new("main").expect("Invalid function name");
        let mut has_fn_info = false;
        let mut fn_definition = MaybeUninit::uninit();
        let handle = unsafe {
            mun_runtime_get_function_info(
                driver.runtime,
                fn_name.as_ptr(),
                &mut has_fn_info as *mut _,
                fn_definition.as_mut_ptr(),
            )
        };
        assert_eq!(handle.0, ptr::null());
        assert!(has_fn_info);
        let _fn_definition = unsafe { fn_definition.assume_init() };
    }

    #[test]
    fn test_runtime_get_type_info_by_name_invalid_type_name() {
        let driver = TestDriver::new(
            r#"
            pub struct Foo;
    "#,
        );

        let handle = unsafe {
            mun_runtime_get_type_info_by_name(
                driver.runtime,
                ptr::null(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'type_name' is null pointer."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_runtime_get_type_info_by_name_invalid_type_name_encoding() {
        let driver = TestDriver::new(
            r#"
            pub struct Foo;
    "#,
        );

        let invalid_encoding = ['�', '\0'];
        let handle = unsafe {
            mun_runtime_get_type_info_by_name(
                driver.runtime,
                invalid_encoding.as_ptr() as *const _,
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'type_name' is not UTF-8 encoded."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_runtime_get_type_info_by_name_invalid_has_type_info() {
        let driver = TestDriver::new(
            r#"
            pub struct Foo;
    "#,
        );

        let type_name = CString::new("Foo").expect("Invalid type name");
        let handle = unsafe {
            mun_runtime_get_type_info_by_name(
                driver.runtime,
                type_name.as_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'has_type_info' is null pointer."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_runtime_get_type_info_by_name_invalid_type_info() {
        let driver = TestDriver::new(
            r#"
            pub struct Foo;
    "#,
        );

        let type_name = CString::new("Foo").expect("Invalid type name");
        let mut has_type_info = false;
        let handle = unsafe {
            mun_runtime_get_type_info_by_name(
                driver.runtime,
                type_name.as_ptr(),
                &mut has_type_info as *mut _,
                ptr::null_mut(),
            )
        };

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'type_info' is null pointer."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_runtime_get_type_info_by_name_none() {
        let driver = TestDriver::new(
            r#"
            pub struct Foo;
    "#,
        );

        let type_name = CString::new("Bar").expect("Invalid type name");
        let mut has_type_info = false;
        let mut type_info = MaybeUninit::uninit();
        let handle = unsafe {
            mun_runtime_get_type_info_by_name(
                driver.runtime,
                type_name.as_ptr(),
                &mut has_type_info as *mut _,
                type_info.as_mut_ptr(),
            )
        };
        assert_eq!(handle.0, ptr::null());
        assert!(!has_type_info);
    }

    #[test]
    fn test_runtime_get_type_info_by_name_some() {
        let driver = TestDriver::new(
            r#"
            pub struct Foo;
    "#,
        );

        let type_name = CString::new("Foo").expect("Invalid type name");
        let mut has_type_info = false;
        let mut type_info = MaybeUninit::uninit();
        let handle = unsafe {
            mun_runtime_get_type_info_by_name(
                driver.runtime,
                type_name.as_ptr(),
                &mut has_type_info as *mut _,
                type_info.as_mut_ptr(),
            )
        };
        assert_eq!(handle.0, ptr::null());
        assert!(has_type_info);
        let _type_info = unsafe { type_info.assume_init() };
    }

    #[test]
    fn test_runtime_get_type_info_by_id_invalid_type_id() {
        let driver = TestDriver::new(
            r#"
            pub struct Foo;
    "#,
        );

        let handle = unsafe {
            mun_runtime_get_type_info_by_id(
                driver.runtime,
                ptr::null(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'type_id' is null pointer."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_runtime_get_type_info_by_id_invalid_has_type_info() {
        let driver = TestDriver::new(
            r#"
            pub struct Foo;
    "#,
        );

        let type_id = abi::TypeId::Concrete(abi::Guid([0; 16]));
        let handle = unsafe {
            mun_runtime_get_type_info_by_id(
                driver.runtime,
                &type_id as *const abi::TypeId,
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'has_type_info' is null pointer."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_runtime_get_type_info_by_id_invalid_type_info() {
        let driver = TestDriver::new(
            r#"
            pub struct Foo;
    "#,
        );

        let type_id = abi::TypeId::Concrete(abi::Guid([0; 16]));
        let mut has_type_info = false;
        let handle = unsafe {
            mun_runtime_get_type_info_by_id(
                driver.runtime,
                &type_id as *const abi::TypeId,
                &mut has_type_info as *mut _,
                ptr::null_mut(),
            )
        };

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'type_info' is null pointer."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_runtime_get_type_info_by_id_none() {
        let driver = TestDriver::new(
            r#"
            pub struct Foo;
    "#,
        );

        let type_id = abi::TypeId::Concrete(abi::Guid([0; 16]));
        let mut has_type_info = false;
        let mut type_info = MaybeUninit::uninit();
        let handle = unsafe {
            mun_runtime_get_type_info_by_id(
                driver.runtime,
                &type_id as *const abi::TypeId,
                &mut has_type_info as *mut _,
                type_info.as_mut_ptr(),
            )
        };
        assert_eq!(handle.0, ptr::null());
        assert!(!has_type_info);
    }

    #[test]
    fn test_runtime_get_type_info_by_id_some() {
        let driver = TestDriver::new(
            r#"
            pub struct Foo;
    "#,
        );

        let type_name = CString::new("Foo").expect("Invalid type name");
        let mut has_type_info = false;
        let mut type_info = MaybeUninit::uninit();
        let handle = unsafe {
            mun_runtime_get_type_info_by_name(
                driver.runtime,
                type_name.as_ptr(),
                &mut has_type_info as *mut _,
                type_info.as_mut_ptr(),
            )
        };
        assert_eq!(handle.0, ptr::null());
        assert!(has_type_info);
    }

    #[test]
    fn test_runtime_update_invalid_updated() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 3 }
    "#,
        );

        let handle = unsafe { mun_runtime_update(driver.runtime, ptr::null_mut()) };

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'updated' is null pointer."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_runtime_update() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 3 }
    "#,
        );

        let mut updated = false;
        let handle = unsafe { mun_runtime_update(driver.runtime, &mut updated as *mut _) };
        assert_eq!(handle.0, ptr::null());
    }
}

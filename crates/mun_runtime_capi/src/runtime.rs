//! Exposes the Mun runtime using the C ABI.

use mun_capi_utils::{
    error::ErrorHandle, mun_error_try, try_convert_c_string, try_deref, try_deref_mut,
};
use mun_memory::{ffi::Type, type_table::TypeTable, Type as RustType};
use mun_runtime::{FunctionDefinition, FunctionPrototype, FunctionSignature};
use std::{ffi::c_void, mem::ManuallyDrop, ops::Deref, os::raw::c_char, slice};
use crate::function::Function;
use mun_abi as abi;

/// A C-style handle to a runtime.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Runtime(pub *mut c_void);

impl Runtime {
    /// Returns a reference to rust Runtime, or an error if this instance contains a null pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the internal pointers point to a valid [`mun_runtime::Runtime`].
    pub(crate) unsafe fn inner(&self) -> Result<&mun_runtime::Runtime, &'static str> {
        (self.0 as *mut mun_runtime::Runtime)
            .as_ref()
            .ok_or("null pointer")
    }

    /// Returns a mutable reference to rust Runtime, or an error if this instance contains a null
    /// pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the internal pointers point to a valid [`mun_runtime::Runtime`].
    pub unsafe fn inner_mut(&self) -> Result<&mut mun_runtime::Runtime, &'static str> {
        (self.0 as *mut mun_runtime::Runtime)
            .as_mut()
            .ok_or("null pointer")
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
    handle: *mut Runtime,
) -> ErrorHandle {
    let library_path = mun_error_try!(try_convert_c_string(library_path)
        .map_err(|e| format!("invalid argument 'library_path': {e}")));
    let handle = try_deref_mut!(handle);

    if options.num_functions > 0 && options.functions.is_null() {
        return ErrorHandle::new("invalid argument: 'functions' is null pointer.");
    }

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
                    Ok(ManuallyDrop::new(ty).deref().clone())
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

    let runtime_options = mun_runtime::RuntimeOptions {
        library_path: library_path.into(),
        user_functions,
        type_table,
    };

    let runtime = match mun_runtime::Runtime::new(runtime_options) {
        Ok(runtime) => runtime,
        Err(e) => return ErrorHandle::new(format!("{:?}", e)),
    };

    handle.0 = Box::into_raw(Box::new(runtime)) as *mut _;
    ErrorHandle::default()
}

/// Destructs the runtime corresponding to `handle`.
#[no_mangle]
pub extern "C" fn mun_runtime_destroy(runtime: Runtime) -> ErrorHandle {
    if runtime.0.is_null() {
        return ErrorHandle::new("invalid argument 'runtime': null pointer");
    }
    let _runtime = unsafe { Box::from_raw(runtime.0) };
    ErrorHandle::default()
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
pub unsafe extern "C" fn mun_runtime_find_function_definition(
    runtime: Runtime,
    fn_name: *const c_char,
    fn_name_len: usize,
    has_fn_info: *mut bool,
    fn_info: *mut Function,
) -> ErrorHandle {
    let runtime = mun_error_try!(runtime
        .inner()
        .map_err(|e| format!("invalid argument 'runtime': {e}")));
    if fn_name.is_null() {
        return ErrorHandle::new("invalid argument 'fn_name': null pointer");
    }
    let name = mun_error_try!(std::str::from_utf8(slice::from_raw_parts(
        fn_name as *const u8,
        fn_name_len
    ))
    .map_err(|_| String::from("invalid argument 'fn_name': invalid UTF-8 encoded")));
    let has_fn_info = try_deref_mut!(has_fn_info);
    let fn_info = try_deref_mut!(fn_info);
    match runtime.get_function_definition(name) {
        Some(info) => {
            *has_fn_info = true;
            *fn_info = info.into()
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
    runtime: Runtime,
    type_name: *const c_char,
    has_type_info: *mut bool,
    type_info: *mut Type,
) -> ErrorHandle {
    let runtime = mun_error_try!(runtime
        .inner()
        .map_err(|e| format!("invalid argument 'runtime': {e}")));
    let type_name =
        mun_error_try!(try_convert_c_string(type_name)
            .map_err(|e| format!("invalid argument 'type_name': {e}")));
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
    runtime: Runtime,
    type_id: *const abi::TypeId,
    has_type_info: *mut bool,
    type_info: *mut Type,
) -> ErrorHandle {
    let runtime = mun_error_try!(runtime
        .inner()
        .map_err(|e| format!("invalid argument 'runtime': {e}")));
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
pub unsafe extern "C" fn mun_runtime_update(runtime: Runtime, updated: *mut bool) -> ErrorHandle {
    let runtime = mun_error_try!(runtime
        .inner_mut()
        .map_err(|e| format!("invalid argument 'runtime': {e}")));
    let updated = try_deref_mut!(updated);
    *updated = runtime.update();
    ErrorHandle::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test_invalid_runtime, test_util::TestDriver};
    use mun_capi_utils::error::mun_error_destroy;
    use mun_capi_utils::{assert_error_snapshot, assert_getter1, assert_getter2, assert_getter3};
    use mun_memory::HasStaticType;
    use std::{ffi::CString, mem::MaybeUninit, ptr};

    test_invalid_runtime!(
        runtime_find_function_definition(ptr::null(), 0, ptr::null_mut(), ptr::null_mut()),
        runtime_get_type_info_by_name(ptr::null(), ptr::null_mut(), ptr::null_mut()),
        runtime_get_type_info_by_id(ptr::null(), ptr::null_mut(), ptr::null_mut()),
        runtime_update(ptr::null_mut())
    );

    #[test]
    fn test_runtime_create_invalid_lib_path() {
        assert_error_snapshot!(
            unsafe { mun_runtime_create(ptr::null(), RuntimeOptions::default(), ptr::null_mut()) },
            @r###""invalid argument \'library_path\': null pointer""###
        );
    }

    #[test]
    fn test_runtime_create_invalid_lib_path_encoding() {
        let invalid_encoding = ['�', '\0'];

        assert_error_snapshot!(
            unsafe {
                mun_runtime_create(
                    invalid_encoding.as_ptr() as *const _,
                    RuntimeOptions::default(),
                    ptr::null_mut(),
                )
            },
            @r###""invalid argument \'library_path\': invalid UTF-8 encoded""###
        );
    }

    #[test]
    fn test_runtime_create_invalid_functions() {
        let lib_path = CString::new("some/path").expect("Invalid library path");

        let options = RuntimeOptions {
            num_functions: 1,
            ..Default::default()
        };

        let mut handle = MaybeUninit::uninit();
        assert_error_snapshot!(
            unsafe { mun_runtime_create(lib_path.into_raw(), options, handle.as_mut_ptr()) },
            @r###""invalid argument: \'functions\' is null pointer.""###
        );
    }

    #[test]
    fn test_runtime_create_invalid_handle() {
        let lib_path = CString::new("some/path").expect("Invalid library path");

        assert_error_snapshot!(
            unsafe {
                mun_runtime_create(lib_path.into_raw(), RuntimeOptions::default(), ptr::null_mut())
            },
            @r###""invalid argument \'handle\': null pointer""###
        );
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

        let mut handle = MaybeUninit::uninit();
        assert_error_snapshot!(
            unsafe { mun_runtime_create(lib_path.into_raw(), options, handle.as_mut_ptr()) },
            @r###""invalid function name: null pointer""###
        );
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

        let mut handle = MaybeUninit::uninit();
        assert_error_snapshot!(
            unsafe { mun_runtime_create(lib_path.into_raw(), options, handle.as_mut_ptr()) },
            @r###""invalid function name: invalid UTF-8 encoded""###
        );
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

        let mut handle = MaybeUninit::uninit();
        assert_error_snapshot!(
            unsafe { mun_runtime_create(lib_path.into_raw(), options, handle.as_mut_ptr()) },
            @r###""invalid function \'foobar\': \'return_type\': null pointer""###
        );
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

        let mut handle = MaybeUninit::uninit();
        assert_error_snapshot!(
            unsafe { mun_runtime_create(lib_path.into_raw(), options, handle.as_mut_ptr()) },
            @r###""invalid function \'foobar\': \'arg_types\' is null pointer.""###
        );
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

        let mut handle = MaybeUninit::uninit();
        assert_error_snapshot!(
            unsafe { mun_runtime_create(lib_path.into_raw(), options, handle.as_mut_ptr()) },
            @r###""invalid function \'foobar\': argument #1: null pointer""###
        );
    }

    #[test]
    fn test_runtime_get_function_info_invalid_fn_name() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 3 }
    "#,
        );

        assert_error_snapshot!(
            unsafe {
                mun_runtime_find_function_definition(
                    driver.runtime,
                    ptr::null(),
                    0,
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            },
            @r###""invalid argument \'fn_name\': null pointer""###
        );
    }

    #[test]
    fn test_runtime_get_function_info_invalid_fn_name_encoding() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 3 }
    "#,
        );

        let invalid_encoding = ['�', '\0'];
        assert_error_snapshot!(
            unsafe {
                mun_runtime_find_function_definition(
                    driver.runtime,
                    invalid_encoding.as_ptr() as *const _,
                    3,
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            },
            @r###""invalid argument \'fn_name\': invalid UTF-8 encoded""###
        );
    }

    #[test]
    fn test_runtime_get_function_info_invalid_has_fn_info() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 3 }
    "#,
        );

        let fn_name = CString::new("main").expect("Invalid function name");
        assert_error_snapshot!(
            unsafe {
                mun_runtime_find_function_definition(
                    driver.runtime,
                    fn_name.as_ptr(),
                    fn_name.as_bytes().len(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            },
            @r###""invalid argument \'has_fn_info\': null pointer""###
        );
    }

    #[test]
    fn test_runtime_get_function_info_invalid_fn_info() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 3 }
    "#,
        );

        let fn_name = CString::new("main").expect("Invalid function name");
        let mut has_fn_info = MaybeUninit::uninit();
        assert_error_snapshot!(
            unsafe {
                mun_runtime_find_function_definition(
                    driver.runtime,
                    fn_name.as_ptr(),
                    fn_name.as_bytes().len(),
                    has_fn_info.as_mut_ptr(),
                    ptr::null_mut(),
                )
            },
            @r###""invalid argument \'fn_info\': null pointer""###
        );
    }

    #[test]
    fn test_runtime_get_function_info_none() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 3 }
    "#,
        );

        let fn_name = CString::new("add").expect("Invalid function name");
        assert_getter3!(mun_runtime_find_function_definition(
            driver.runtime,
            fn_name.as_ptr(),
            fn_name.as_bytes().len(),
            has_fn_info,
            _fn_definition,
        ));
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
        assert_getter3!(mun_runtime_find_function_definition(
            driver.runtime,
            fn_name.as_ptr(),
            fn_name.as_bytes().len(),
            has_fn_info,
            _fn_definition,
        ));
        assert!(has_fn_info);
    }

    #[test]
    fn test_runtime_get_type_info_by_name_invalid_type_name() {
        let driver = TestDriver::new(
            r#"
            pub struct Foo;
    "#,
        );

        assert_error_snapshot!(
            unsafe {
                mun_runtime_get_type_info_by_name(
                    driver.runtime,
                    ptr::null(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            },
            @r###""invalid argument \'type_name\': null pointer""###
        );
    }

    #[test]
    fn test_runtime_get_type_info_by_name_invalid_type_name_encoding() {
        let driver = TestDriver::new(
            r#"
            pub struct Foo;
    "#,
        );

        let invalid_encoding = ['�', '\0'];
        assert_error_snapshot!(
            unsafe {
                mun_runtime_get_type_info_by_name(
                    driver.runtime,
                    invalid_encoding.as_ptr() as *const _,
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            },
            @r###""invalid argument \'type_name\': invalid UTF-8 encoded""###
        );
    }

    #[test]
    fn test_runtime_get_type_info_by_name_invalid_has_type_info() {
        let driver = TestDriver::new(
            r#"
            pub struct Foo;
    "#,
        );

        let type_name = CString::new("Foo").expect("Invalid type name");
        assert_error_snapshot!(
            unsafe {
                mun_runtime_get_type_info_by_name(
                    driver.runtime,
                    type_name.as_ptr(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            },
            @r###""invalid argument \'has_type_info\': null pointer""###
        );
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
        assert_error_snapshot!(
            unsafe {
                mun_runtime_get_type_info_by_name(
                    driver.runtime,
                    type_name.as_ptr(),
                    &mut has_type_info as *mut _,
                    ptr::null_mut(),
                )
            },
            @r###""invalid argument \'type_info\': null pointer""###
        );
    }

    #[test]
    fn test_runtime_get_type_info_by_name_none() {
        let driver = TestDriver::new(
            r#"
            pub struct Foo;
    "#,
        );

        let type_name = CString::new("Bar").expect("Invalid type name");
        assert_getter2!(mun_runtime_get_type_info_by_name(
            driver.runtime,
            type_name.as_ptr(),
            has_type_info,
            _type_info,
        ));
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
        assert_getter2!(mun_runtime_get_type_info_by_name(
            driver.runtime,
            type_name.as_ptr(),
            has_type_info,
            _type_info,
        ));
        assert!(has_type_info);
    }

    #[test]
    fn test_runtime_get_type_info_by_id_invalid_type_id() {
        let driver = TestDriver::new(
            r#"
            pub struct Foo;
    "#,
        );

        assert_error_snapshot!(
            unsafe {
                mun_runtime_get_type_info_by_id(
                    driver.runtime,
                    ptr::null(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            },
            @r###""invalid argument \'type_id\': null pointer""###
        );
    }

    #[test]
    fn test_runtime_get_type_info_by_id_invalid_has_type_info() {
        let driver = TestDriver::new(
            r#"
            pub struct Foo;
    "#,
        );

        let type_id = abi::TypeId::Concrete(abi::Guid([0; 16]));
        assert_error_snapshot!(
            unsafe {
                mun_runtime_get_type_info_by_id(
                    driver.runtime,
                    &type_id as *const abi::TypeId,
                    ptr::null_mut(),
                    ptr::null_mut(),
                )
            },
            @r###""invalid argument \'has_type_info\': null pointer""###
        );
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
        assert_error_snapshot!(
            unsafe {
                mun_runtime_get_type_info_by_id(
                    driver.runtime,
                    &type_id as *const abi::TypeId,
                    &mut has_type_info as *mut _,
                    ptr::null_mut(),
                )
            },
            @r###""invalid argument \'type_info\': null pointer""###
        );
    }

    #[test]
    fn test_runtime_get_type_info_by_id_none() {
        let driver = TestDriver::new(
            r#"
            pub struct Foo;
    "#,
        );

        let type_id = abi::TypeId::Concrete(abi::Guid([0; 16]));
        assert_getter2!(mun_runtime_get_type_info_by_id(
            driver.runtime,
            &type_id as *const abi::TypeId,
            has_type_info,
            _type_info,
        ));
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
        assert_getter2!(mun_runtime_get_type_info_by_name(
            driver.runtime,
            type_name.as_ptr(),
            has_type_info,
            _type_info,
        ));
        assert!(has_type_info);
    }

    #[test]
    fn test_runtime_update_invalid_updated() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 3 }
    "#,
        );

        assert_error_snapshot!(
            unsafe { mun_runtime_update(driver.runtime, ptr::null_mut()) },
            @r###""invalid argument \'updated\': null pointer""###
        );
    }

    #[test]
    fn test_runtime_update() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 3 }
    "#,
        );

        assert_getter1!(mun_runtime_update(driver.runtime, _updated));
    }
}

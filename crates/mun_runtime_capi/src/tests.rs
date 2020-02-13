use crate::{error::*, *};
use mun_compiler::{Config, Driver, PathOrInline, RelativePathBuf};
use std::{ffi::CString, mem, path::Path, ptr};

use std::io::stderr;

/// Combines a compiler and runtime in one. Use of the TestDriver allows for quick testing of Mun
/// constructs in the runtime with hot-reloading support.
struct TestDriver {
    _temp_dir: tempfile::TempDir,
    runtime: RuntimeHandle,
}

impl TestDriver {
    /// Constructs a new `TestDriver` from Mun source
    fn new(text: &str) -> Self {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config = Config {
            out_dir: Some(temp_dir.path().to_path_buf()),
            ..Config::default()
        };
        let input = PathOrInline::Inline {
            rel_path: RelativePathBuf::from("main.mun"),
            contents: text.to_owned(),
        };
        let (mut driver, file_id) = Driver::with_file(config, input).unwrap();
        if driver.emit_diagnostics(&mut stderr()).unwrap() {
            panic!("compiler errors..")
        }
        let out_path = driver.write_assembly(file_id).unwrap();
        let runtime = make_runtime(&out_path);
        TestDriver {
            _temp_dir: temp_dir,
            runtime,
        }
    }
}

impl Drop for TestDriver {
    fn drop(&mut self) {
        mun_runtime_destroy(self.runtime);
    }
}

fn make_runtime(lib_path: &Path) -> RuntimeHandle {
    let lib_path = lib_path.to_str().expect("Invalid lib path");
    let lib_path = CString::new(lib_path).unwrap();

    let mut handle = RuntimeHandle(ptr::null_mut());
    let error = unsafe { mun_runtime_create(lib_path.as_ptr(), &mut handle as *mut _) };
    assert_eq!(error.token(), 0, "Failed to create runtime");
    handle
}

#[test]
fn test_runtime_create_invalid_lib_path() {
    let handle = unsafe { mun_runtime_create(ptr::null(), ptr::null_mut()) };
    assert_ne!(handle.token(), 0);

    let message = unsafe { CStr::from_ptr(mun_error_message(handle)) };
    assert_eq!(
        message.to_str().unwrap(),
        "Invalid argument: 'library_path' is null pointer."
    );

    unsafe { mun_destroy_string(message.as_ptr()) };
}

#[test]
fn test_runtime_create_invalid_lib_path_encoding() {
    let invalid_encoding = ['�', '\0'];

    let handle =
        unsafe { mun_runtime_create(invalid_encoding.as_ptr() as *const _, ptr::null_mut()) };
    assert_ne!(handle.token(), 0);

    let message = unsafe { CStr::from_ptr(mun_error_message(handle)) };
    assert_eq!(
        message.to_str().unwrap(),
        "Invalid argument: 'library_path' is not UTF-8 encoded."
    );

    unsafe { mun_destroy_string(message.as_ptr()) };
}

#[test]
fn test_runtime_create_invalid_handle() {
    let lib_path = CString::new("some/path").expect("Invalid library path");

    let handle = unsafe { mun_runtime_create(lib_path.into_raw(), ptr::null_mut()) };
    assert_ne!(handle.token(), 0);

    let message = unsafe { CStr::from_ptr(mun_error_message(handle)) };
    assert_eq!(
        message.to_str().unwrap(),
        "Invalid argument: 'handle' is null pointer."
    );

    unsafe { mun_destroy_string(message.as_ptr()) };
}

#[test]
fn test_runtime_get_function_info_invalid_runtime() {
    let runtime = RuntimeHandle(ptr::null_mut());
    let handle = unsafe {
        mun_runtime_get_function_info(runtime, ptr::null(), ptr::null_mut(), ptr::null_mut())
    };

    let message = unsafe { CStr::from_ptr(mun_error_message(handle)) };
    assert_eq!(
        message.to_str().unwrap(),
        "Invalid argument: 'runtime' is null pointer."
    );

    unsafe { mun_destroy_string(message.as_ptr()) };
}

#[test]
fn test_runtime_get_function_info_invalid_fn_name() {
    let driver = TestDriver::new(
        r#"
        pub fn main() -> int { 3 }
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

    let message = unsafe { CStr::from_ptr(mun_error_message(handle)) };
    assert_eq!(
        message.to_str().unwrap(),
        "Invalid argument: 'fn_name' is null pointer."
    );

    unsafe { mun_destroy_string(message.as_ptr()) };
}

#[test]
fn test_runtime_get_function_info_invalid_fn_name_encoding() {
    let driver = TestDriver::new(
        r#"
        pub fn main() -> int { 3 }
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

    let message = unsafe { CStr::from_ptr(mun_error_message(handle)) };
    assert_eq!(
        message.to_str().unwrap(),
        "Invalid argument: 'fn_name' is not UTF-8 encoded."
    );

    unsafe { mun_destroy_string(message.as_ptr()) };
}

#[test]
fn test_runtime_get_function_info_invalid_has_fn_info() {
    let driver = TestDriver::new(
        r#"
        pub fn main() -> int { 3 }
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

    let message = unsafe { CStr::from_ptr(mun_error_message(handle)) };
    assert_eq!(
        message.to_str().unwrap(),
        "Invalid argument: 'has_fn_info' is null pointer."
    );

    unsafe { mun_destroy_string(message.as_ptr()) };
}

#[test]
fn test_runtime_get_function_info_invalid_fn_info() {
    let driver = TestDriver::new(
        r#"
        pub fn main() -> int { 3 }
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

    let message = unsafe { CStr::from_ptr(mun_error_message(handle)) };
    assert_eq!(
        message.to_str().unwrap(),
        "Invalid argument: 'fn_info' is null pointer."
    );

    unsafe { mun_destroy_string(message.as_ptr()) };
}

#[test]
fn test_runtime_get_function_info() {
    let driver = TestDriver::new(
        r#"
        pub fn main() -> int { 3 }
    "#,
    );

    let fn_name = CString::new("main").expect("Invalid function name");
    let mut has_fn_info = false;
    let mut fn_info = unsafe { mem::zeroed::<FunctionInfo>() };
    let handle = unsafe {
        mun_runtime_get_function_info(
            driver.runtime,
            fn_name.as_ptr(),
            &mut has_fn_info as *mut _,
            &mut fn_info as *mut _,
        )
    };
    assert_eq!(handle.token(), 0);
}

#[test]
fn test_runtime_update_invalid_runtime() {
    let runtime = RuntimeHandle(ptr::null_mut());
    let handle = unsafe { mun_runtime_update(runtime, ptr::null_mut()) };

    let message = unsafe { CStr::from_ptr(mun_error_message(handle)) };
    assert_eq!(
        message.to_str().unwrap(),
        "Invalid argument: 'runtime' is null pointer."
    );

    unsafe { mun_destroy_string(message.as_ptr()) };
}

#[test]
fn test_runtime_update_invalid_updated() {
    let driver = TestDriver::new(
        r#"
        pub fn main() -> int { 3 }
    "#,
    );

    let handle = unsafe { mun_runtime_update(driver.runtime, ptr::null_mut()) };

    let message = unsafe { CStr::from_ptr(mun_error_message(handle)) };
    assert_eq!(
        message.to_str().unwrap(),
        "Invalid argument: 'updated' is null pointer."
    );

    unsafe { mun_destroy_string(message.as_ptr()) };
}

#[test]
fn test_runtime_update() {
    let driver = TestDriver::new(
        r#"
        pub fn main() -> int { 3 }
    "#,
    );

    let mut updated = false;
    let handle = unsafe { mun_runtime_update(driver.runtime, &mut updated as *mut _) };
    assert_eq!(handle.token(), 0);
}

#[test]
fn test_type_info_as_struct_invalid_type_info() {
    let handle = unsafe { mun_type_info_as_struct(ptr::null(), ptr::null_mut()) };

    let message = unsafe { CStr::from_ptr(mun_error_message(handle)) };
    assert_eq!(
        message.to_str().unwrap(),
        "Invalid argument: 'type_info' is null pointer."
    );

    unsafe { mun_destroy_string(message.as_ptr()) };
}

#[test]
fn test_type_info_as_struct_invalid_struct_info() {
    let type_info = unsafe { mem::zeroed::<TypeInfo>() };
    let handle = unsafe { mun_type_info_as_struct(&type_info as *const _, ptr::null_mut()) };

    let message = unsafe { CStr::from_ptr(mun_error_message(handle)) };
    assert_eq!(
        message.to_str().unwrap(),
        "Invalid argument: 'struct_info' is null pointer."
    );

    unsafe { mun_destroy_string(message.as_ptr()) };
}

#[test]
fn test_type_info_as_struct_not_a_struct() {
    let driver = TestDriver::new(
        r#"
        pub fn main() -> int { 3 }
    "#,
    );

    let fn_name = CString::new("main").expect("Invalid function name");
    let mut has_fn_info = false;
    let mut fn_info = unsafe { mem::zeroed::<FunctionInfo>() };
    let handle = unsafe {
        mun_runtime_get_function_info(
            driver.runtime,
            fn_name.as_ptr(),
            &mut has_fn_info as *mut _,
            &mut fn_info as *mut _,
        )
    };
    assert_eq!(handle.token(), 0);
    assert!(has_fn_info);

    let mut struct_info = unsafe { mem::zeroed::<StructInfo>() };
    let handle = unsafe {
        mun_type_info_as_struct(fn_info.signature.return_type, &mut struct_info as *mut _)
    };

    let message = unsafe { CStr::from_ptr(mun_error_message(handle)) };
    assert_eq!(message.to_str().unwrap(), "`core::i64` is not a struct.");

    unsafe { mun_destroy_string(message.as_ptr()) };
}

#[test]
fn test_type_info_as_struct() {
    let driver = TestDriver::new(
        r#"
        struct Foo;

        pub fn main() -> Foo { Foo }
    "#,
    );

    let fn_name = CString::new("main").expect("Invalid function name");
    let mut has_fn_info = false;
    let mut fn_info = unsafe { mem::zeroed::<FunctionInfo>() };
    let handle = unsafe {
        mun_runtime_get_function_info(
            driver.runtime,
            fn_name.as_ptr(),
            &mut has_fn_info as *mut _,
            &mut fn_info as *mut _,
        )
    };
    assert_eq!(handle.token(), 0);
    assert!(has_fn_info);

    let mut struct_info = unsafe { mem::zeroed::<StructInfo>() };
    let handle = unsafe {
        mun_type_info_as_struct(fn_info.signature.return_type, &mut struct_info as *mut _)
    };
    assert_eq!(handle.token(), 0);
}

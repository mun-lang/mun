use crate::{error::*, gc::*, *};
use compiler::{Config, Driver, PathOrInline, RelativePathBuf};
use memory::gc::{GcPtr, HasIndirectionPtr, RawGcPtr};
use runtime::UnsafeTypeInfo;
use std::{
    ffi::CString,
    io::stderr,
    mem::{self, MaybeUninit},
    path::Path,
    ptr::{self, NonNull},
};

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

macro_rules! test_invalid_runtime {
    ($(
        $name:ident($($arg:expr),*)
    ),+) => {
        $(
            paste::item! {
                #[test]
                fn [<test_ $name _invalid_runtime>]() {
                    let runtime = RuntimeHandle(ptr::null_mut());
                    let handle =
                        unsafe { [<mun_ $name>](runtime $(, $arg)*) };

                    let message = unsafe { CStr::from_ptr(mun_error_message(handle)) };
                    assert_eq!(
                        message.to_str().unwrap(),
                        "Invalid argument: 'runtime' is null pointer."
                    );

                    unsafe { mun_destroy_string(message.as_ptr()) };
                }
            }
        )+
    };
}

test_invalid_runtime!(
    runtime_get_function_info(ptr::null(), ptr::null_mut(), ptr::null_mut()),
    runtime_update(ptr::null_mut()),
    gc_alloc(UnsafeTypeInfo::new(NonNull::dangling()), ptr::null_mut()),
    gc_ptr_type(mem::zeroed::<GcPtr>(), ptr::null_mut()),
    gc_root(mem::zeroed::<GcPtr>()),
    gc_unroot(mem::zeroed::<GcPtr>()),
    gc_collect(ptr::null_mut())
);

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
        pub fn main() -> i32 { 3 }
    "#,
    );

    let fn_name = CString::new("main").expect("Invalid function name");
    let mut has_fn_info = false;
    let mut fn_info = MaybeUninit::uninit();
    let handle = unsafe {
        mun_runtime_get_function_info(
            driver.runtime,
            fn_name.as_ptr(),
            &mut has_fn_info as *mut _,
            fn_info.as_mut_ptr(),
        )
    };
    assert_eq!(handle.token(), 0);
    assert!(has_fn_info);
    let _fn_info = unsafe { fn_info.assume_init() };
}

#[test]
fn test_runtime_update_invalid_updated() {
    let driver = TestDriver::new(
        r#"
        pub fn main() -> i32 { 3 }
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
        pub fn main() -> i32 { 3 }
    "#,
    );

    let mut updated = false;
    let handle = unsafe { mun_runtime_update(driver.runtime, &mut updated as *mut _) };
    assert_eq!(handle.token(), 0);
}

#[test]
fn test_gc_alloc_invalid_obj() {
    let driver = TestDriver::new(
        r#"
        struct Foo;

        pub fn main() -> Foo { Foo }
    "#,
    );
    let fn_name = CString::new("main").expect("Invalid function name");
    let mut has_fn_info = false;
    let mut fn_info = MaybeUninit::uninit();
    let handle = unsafe {
        mun_runtime_get_function_info(
            driver.runtime,
            fn_name.as_ptr(),
            &mut has_fn_info as *mut _,
            fn_info.as_mut_ptr(),
        )
    };
    assert_eq!(handle.token(), 0);

    let fn_info = unsafe { fn_info.assume_init() };
    // TODO: Simplify this once we have `mun_runtime_find_type_info`
    let return_type = fn_info.signature.return_type().unwrap();
    let return_type =
        UnsafeTypeInfo::new(NonNull::new(return_type as *const abi::TypeInfo as *mut _).unwrap());

    let handle = unsafe { mun_gc_alloc(driver.runtime, return_type, ptr::null_mut()) };
    let message = unsafe { CStr::from_ptr(mun_error_message(handle)) };
    assert_eq!(
        message.to_str().unwrap(),
        "Invalid argument: 'obj' is null pointer."
    );

    unsafe { mun_destroy_string(message.as_ptr()) };
}

#[test]
fn test_gc_alloc() {
    let driver = TestDriver::new(
        r#"
        struct Foo;

        pub fn main() -> Foo { Foo }
    "#,
    );
    let fn_name = CString::new("main").expect("Invalid function name");
    let mut has_fn_info = false;
    let mut fn_info = MaybeUninit::uninit();
    let handle = unsafe {
        mun_runtime_get_function_info(
            driver.runtime,
            fn_name.as_ptr(),
            &mut has_fn_info as *mut _,
            fn_info.as_mut_ptr(),
        )
    };
    assert_eq!(handle.token(), 0);

    let fn_info = unsafe { fn_info.assume_init() };
    // TODO: Simplify this once we have `mun_runtime_find_type_info`
    let return_type = fn_info.signature.return_type().unwrap();
    let return_type =
        UnsafeTypeInfo::new(NonNull::new(return_type as *const abi::TypeInfo as *mut _).unwrap());

    let mut obj = MaybeUninit::uninit();
    let handle = unsafe { mun_gc_alloc(driver.runtime, return_type, obj.as_mut_ptr()) };
    assert_eq!(handle.token(), 0);

    let obj = unsafe { obj.assume_init() };
    assert_ne!(unsafe { obj.deref::<u8>() }, ptr::null());

    let mut reclaimed = false;
    let handle = unsafe { mun_gc_collect(driver.runtime, &mut reclaimed as *mut _) };
    assert_eq!(handle.token(), 0);
}

#[test]
fn test_gc_ptr_type_invalid_type_info() {
    let driver = TestDriver::new(
        r#"
        struct Foo;

        pub fn main() -> Foo { Foo }
    "#,
    );

    let handle = unsafe {
        let raw_ptr: RawGcPtr = ptr::null();
        mun_gc_ptr_type(driver.runtime, raw_ptr.into(), ptr::null_mut())
    };

    let message = unsafe { CStr::from_ptr(mun_error_message(handle)) };
    assert_eq!(
        message.to_str().unwrap(),
        "Invalid argument: 'type_info' is null pointer."
    );

    unsafe { mun_destroy_string(message.as_ptr()) };
}

#[test]
fn test_gc_ptr_type() {
    let driver = TestDriver::new(
        r#"
        struct Foo;

        pub fn main() -> Foo { Foo }
    "#,
    );
    let fn_name = CString::new("main").expect("Invalid function name");
    let mut has_fn_info = false;
    let mut fn_info = MaybeUninit::uninit();
    let handle = unsafe {
        mun_runtime_get_function_info(
            driver.runtime,
            fn_name.as_ptr(),
            &mut has_fn_info as *mut _,
            fn_info.as_mut_ptr(),
        )
    };
    assert_eq!(handle.token(), 0);

    let fn_info = unsafe { fn_info.assume_init() };
    // TODO: Simplify this once we have `mun_runtime_find_type_info`
    let return_type = fn_info.signature.return_type().unwrap();
    let return_type =
        UnsafeTypeInfo::new(NonNull::new(return_type as *const abi::TypeInfo as *mut _).unwrap());

    let mut obj = MaybeUninit::uninit();
    let handle = unsafe { mun_gc_alloc(driver.runtime, return_type, obj.as_mut_ptr()) };
    assert_eq!(handle.token(), 0);

    let obj = unsafe { obj.assume_init() };
    assert_ne!(unsafe { obj.deref::<u8>() }, ptr::null());

    let mut ty = MaybeUninit::uninit();
    let handle = unsafe { mun_gc_ptr_type(driver.runtime, obj, ty.as_mut_ptr()) };
    assert_eq!(handle.token(), 0);

    let _ty = unsafe { ty.assume_init() };

    let mut reclaimed = false;
    let handle = unsafe { mun_gc_collect(driver.runtime, &mut reclaimed as *mut _) };
    assert_eq!(handle.token(), 0);
    assert!(reclaimed);
}

#[test]
fn test_gc_rooting() {
    let driver = TestDriver::new(
        r#"
        struct Foo;

        pub fn main() -> Foo { Foo }
    "#,
    );
    let fn_name = CString::new("main").expect("Invalid function name");
    let mut has_fn_info = false;
    let mut fn_info = MaybeUninit::uninit();
    let handle = unsafe {
        mun_runtime_get_function_info(
            driver.runtime,
            fn_name.as_ptr(),
            &mut has_fn_info as *mut _,
            fn_info.as_mut_ptr(),
        )
    };
    assert_eq!(handle.token(), 0);

    let fn_info = unsafe { fn_info.assume_init() };
    // TODO: Simplify this once we have `mun_runtime_find_type_info`
    let return_type = fn_info.signature.return_type().unwrap();
    let return_type =
        UnsafeTypeInfo::new(NonNull::new(return_type as *const abi::TypeInfo as *mut _).unwrap());

    let mut obj = MaybeUninit::uninit();
    let handle = unsafe { mun_gc_alloc(driver.runtime, return_type, obj.as_mut_ptr()) };
    assert_eq!(handle.token(), 0);

    let obj = unsafe { obj.assume_init() };
    assert_ne!(unsafe { obj.deref::<u8>() }, ptr::null());

    let handle = unsafe { mun_gc_root(driver.runtime, obj) };
    assert_eq!(handle.token(), 0);

    let mut reclaimed = false;
    let handle = unsafe { mun_gc_collect(driver.runtime, &mut reclaimed as *mut _) };
    assert_eq!(handle.token(), 0);
    assert!(!reclaimed);

    let handle = unsafe { mun_gc_unroot(driver.runtime, obj) };
    assert_eq!(handle.token(), 0);

    let handle = unsafe { mun_gc_collect(driver.runtime, &mut reclaimed as *mut _) };
    assert_eq!(handle.token(), 0);
    assert!(reclaimed);
}

#[test]
fn test_gc_ptr_collect_invalid_reclaimed() {
    let driver = TestDriver::new(
        r#"
        struct Foo;

        pub fn main() -> Foo { Foo }
    "#,
    );

    let handle = unsafe { mun_gc_collect(driver.runtime, ptr::null_mut()) };

    let message = unsafe { CStr::from_ptr(mun_error_message(handle)) };
    assert_eq!(
        message.to_str().unwrap(),
        "Invalid argument: 'reclaimed' is null pointer."
    );

    unsafe { mun_destroy_string(message.as_ptr()) };
}

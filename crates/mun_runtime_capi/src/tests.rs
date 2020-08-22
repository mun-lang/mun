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
        let out_path = driver.assembly_output_path(file_id);
        driver.write_assembly(file_id, true).unwrap();
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
    let error = unsafe {
        mun_runtime_create(
            lib_path.as_ptr(),
            RuntimeOptions::default(),
            &mut handle as *mut _,
        )
    };
    assert_eq!(error.token(), 0, "Failed to create runtime");
    handle
}

fn assert_error_message_eq(handle: ErrorHandle, expected_message: &str) {
    assert_ne!(handle.token(), 0);

    let message = unsafe { CStr::from_ptr(mun_error_message(handle)) };
    assert_eq!(message.to_str().unwrap(), expected_message);

    unsafe { mun_destroy_string(message.as_ptr()) };
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

                    assert_error_message_eq(
                        handle,
                        "Invalid argument: 'runtime' is null pointer."
                    );
                }
            }
        )+
    };
}

test_invalid_runtime!(
    runtime_get_function_definition(ptr::null(), ptr::null_mut(), ptr::null_mut()),
    runtime_find_type_info_by_guid(ptr::null(), ptr::null_mut(), ptr::null_mut()),
    runtime_find_type_info_by_name(ptr::null(), ptr::null_mut(), ptr::null_mut()),
    runtime_update(ptr::null_mut()),
    gc_alloc(UnsafeTypeInfo::new(NonNull::dangling()), ptr::null_mut()),
    gc_ptr_type(mem::zeroed::<GcPtr>(), ptr::null_mut()),
    gc_root(mem::zeroed::<GcPtr>()),
    gc_unroot(mem::zeroed::<GcPtr>()),
    gc_collect(ptr::null_mut())
);

#[test]
fn test_runtime_create_invalid_lib_path() {
    let handle =
        unsafe { mun_runtime_create(ptr::null(), RuntimeOptions::default(), ptr::null_mut()) };

    assert_error_message_eq(handle, "Invalid argument: 'library_path' is null pointer.");
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

    assert_error_message_eq(
        handle,
        "Invalid argument: 'library_path' is not UTF-8 encoded.",
    );
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

    assert_error_message_eq(handle, "Invalid argument: 'handle' is null pointer.");
}

#[test]
fn test_runtime_get_function_info_invalid_fn_name() {
    let driver = TestDriver::new(
        r#"
        pub fn main() -> i32 { 3 }
    "#,
    );

    let handle = unsafe {
        mun_runtime_get_function_definition(
            driver.runtime,
            ptr::null(),
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };

    assert_error_message_eq(handle, "Invalid argument: 'fn_name' is null pointer.");
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
        mun_runtime_get_function_definition(
            driver.runtime,
            invalid_encoding.as_ptr() as *const _,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };

    assert_error_message_eq(handle, "Invalid argument: 'fn_name' is not UTF-8 encoded.");
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
        mun_runtime_get_function_definition(
            driver.runtime,
            fn_name.as_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };

    assert_error_message_eq(handle, "Invalid argument: 'has_fn_info' is null pointer.");
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
        mun_runtime_get_function_definition(
            driver.runtime,
            fn_name.as_ptr(),
            &mut has_fn_info as *mut _,
            ptr::null_mut(),
        )
    };

    assert_error_message_eq(handle, "Invalid argument: 'fn_info' is null pointer.");
}

#[test]
fn test_runtime_get_function_info() {
    let driver = TestDriver::new(
        r#"
        pub fn main() -> i32 { 3 }
    "#,
    );

    let name = "main";
    let fn_name = CString::new(name).expect("Invalid function name");
    let mut has_fn_info = false;
    let mut fn_definition = MaybeUninit::uninit();
    let handle = unsafe {
        mun_runtime_get_function_definition(
            driver.runtime,
            fn_name.as_ptr(),
            &mut has_fn_info as *mut _,
            fn_definition.as_mut_ptr(),
        )
    };
    assert_eq!(handle.token(), 0);
    assert!(has_fn_info);

    let fn_definition = unsafe { fn_definition.assume_init() };
    assert_eq!(fn_definition.prototype.name(), name);
}

#[test]
fn test_runtime_find_type_info_by_guid_invalid_type_guid() {
    let driver = TestDriver::new(
        r#"
        pub fn main() -> i32 { 3 }
    "#,
    );

    let handle = unsafe {
        mun_runtime_find_type_info_by_name(
            driver.runtime,
            ptr::null(),
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };

    assert_error_message_eq(handle, "Invalid argument: 'type_guid' is null pointer.");
}

#[test]
fn test_runtime_find_type_info_by_guid_invalid_has_type_info() {
    let driver = TestDriver::new(
        r#"
        pub fn main() -> i32 { 3 }
    "#,
    );

    let type_guid = abi::Guid([0u8; 16]);
    let handle = unsafe {
        mun_runtime_find_type_info_by_guid(
            driver.runtime,
            &type_guid as *const _,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };

    assert_error_message_eq(handle, "Invalid argument: 'has_type_info' is null pointer.");
}

#[test]
fn test_runtime_find_type_info_by_guid_invalid_type_info() {
    let driver = TestDriver::new(
        r#"
        pub fn main() -> i32 { 3 }
    "#,
    );

    let type_guid = abi::Guid([0u8; 16]);
    let mut has_type_info = false;
    let handle = unsafe {
        mun_runtime_find_type_info_by_guid(
            driver.runtime,
            &type_guid as *const _,
            &mut has_type_info as *mut _,
            ptr::null_mut(),
        )
    };

    assert_error_message_eq(handle, "Invalid argument: 'type_info' is null pointer.");
}

// TODO: Once we can derive Rust-wrappers (with GUID) for Mun structs
// TODO: Add primitive types to runtime?
// fn test_runtime_find_type_info_by_guid() {}

#[test]
fn test_runtime_find_type_info_by_name_invalid_type_name() {
    let driver = TestDriver::new(
        r#"
        pub fn main() -> i32 { 3 }
    "#,
    );

    let handle = unsafe {
        mun_runtime_find_type_info_by_name(
            driver.runtime,
            ptr::null(),
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };

    assert_error_message_eq(handle, "Invalid argument: 'type_name' is null pointer.");
}

#[test]
fn test_runtime_find_type_info_by_name_invalid_type_name_encoding() {
    let driver = TestDriver::new(
        r#"
        pub fn main() -> i32 { 3 }
    "#,
    );

    let invalid_encoding = ['�', '\0'];
    let handle = unsafe {
        mun_runtime_find_type_info_by_name(
            driver.runtime,
            invalid_encoding.as_ptr() as *const _,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };

    assert_error_message_eq(
        handle,
        "Invalid argument: 'type_name' is not UTF-8 encoded.",
    );
}

#[test]
fn test_runtime_find_type_info_by_name_invalid_has_type_info() {
    let driver = TestDriver::new(
        r#"
        pub fn main() -> i32 { 3 }
    "#,
    );

    let type_name = CString::new("main").expect("Invalid type name");
    let handle = unsafe {
        mun_runtime_find_type_info_by_name(
            driver.runtime,
            type_name.as_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };

    assert_error_message_eq(handle, "Invalid argument: 'has_type_info' is null pointer.");
}

#[test]
fn test_runtime_find_type_info_by_name_invalid_type_info() {
    let driver = TestDriver::new(
        r#"
        pub fn main() -> i32 { 3 }
    "#,
    );

    let type_name = CString::new("main").expect("Invalid type name");
    let mut has_type_info = false;
    let handle = unsafe {
        mun_runtime_find_type_info_by_name(
            driver.runtime,
            type_name.as_ptr(),
            &mut has_type_info as *mut _,
            ptr::null_mut(),
        )
    };

    assert_error_message_eq(handle, "Invalid argument: 'type_info' is null pointer.");
}

#[test]
fn test_runtime_find_type_info_by_name() {
    let driver = TestDriver::new(
        r#"
        struct Foo;

        pub fn main() -> Foo { Foo }
    "#,
    );

    let name = "main";
    let type_name = CString::new(name).expect("Invalid type name");
    let mut has_type_info = false;
    let mut type_info = MaybeUninit::uninit();
    let handle = unsafe {
        mun_runtime_find_type_info_by_name(
            driver.runtime,
            type_name.as_ptr(),
            &mut has_type_info as *mut _,
            type_info.as_mut_ptr(),
        )
    };
    assert_eq!(handle.token(), 0);
    assert!(has_type_info);

    let type_info = unsafe { type_info.assume_init() };
    assert_eq!(type_info.name(), name);
}

#[test]
fn test_runtime_update_invalid_updated() {
    let driver = TestDriver::new(
        r#"
        pub fn main() -> i32 { 3 }
    "#,
    );

    let handle = unsafe { mun_runtime_update(driver.runtime, ptr::null_mut()) };

    assert_error_message_eq(handle, "Invalid argument: 'updated' is null pointer.");
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

    let type_name = CString::new("Foo").expect("Invalid type name");
    let mut has_type_info = false;
    let mut type_info = MaybeUninit::uninit();
    let handle = unsafe {
        mun_runtime_find_type_info_by_name(
            driver.runtime,
            type_name.as_ptr(),
            &mut has_type_info as *mut _,
            type_info.as_mut_ptr(),
        )
    };
    assert_eq!(handle.token(), 0);

    let type_info = unsafe { type_info.assume_init() };

    let handle = unsafe {
        mun_gc_alloc(
            driver.runtime,
            UnsafeTypeInfo::from(&type_info),
            ptr::null_mut(),
        )
    };
    assert_error_message_eq(handle, "Invalid argument: 'obj' is null pointer.");
}

#[test]
fn test_gc_alloc() {
    let driver = TestDriver::new(
        r#"
        struct Foo;

        pub fn main() -> Foo { Foo }
    "#,
    );

    let type_name = CString::new("Foo").expect("Invalid type name");
    let mut has_type_info = false;
    let mut type_info = MaybeUninit::uninit();
    let handle = unsafe {
        mun_runtime_find_type_info_by_name(
            driver.runtime,
            type_name.as_ptr(),
            &mut has_type_info as *mut _,
            type_info.as_mut_ptr(),
        )
    };
    assert_eq!(handle.token(), 0);

    let type_info = unsafe { type_info.assume_init() };

    let mut obj = MaybeUninit::uninit();
    let handle = unsafe {
        mun_gc_alloc(
            driver.runtime,
            UnsafeTypeInfo::from(&type_info),
            obj.as_mut_ptr(),
        )
    };
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
    assert_error_message_eq(handle, "Invalid argument: 'type_info' is null pointer.");
}

#[test]
fn test_gc_ptr_type() {
    let driver = TestDriver::new(
        r#"
        struct Foo;

        pub fn main() -> Foo { Foo }
    "#,
    );

    let type_name = CString::new("Foo").expect("Invalid type name");
    let mut has_type_info = false;
    let mut type_info = MaybeUninit::uninit();
    let handle = unsafe {
        mun_runtime_find_type_info_by_name(
            driver.runtime,
            type_name.as_ptr(),
            &mut has_type_info as *mut _,
            type_info.as_mut_ptr(),
        )
    };
    assert_eq!(handle.token(), 0);

    let type_info = unsafe { type_info.assume_init() };

    let mut obj = MaybeUninit::uninit();
    let handle = unsafe {
        mun_gc_alloc(
            driver.runtime,
            UnsafeTypeInfo::from(&type_info),
            obj.as_mut_ptr(),
        )
    };
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
fn test_gc_collect_invalid_reclaimed() {
    let driver = TestDriver::new(
        r#"
        struct Foo;

        pub fn main() -> Foo { Foo }
    "#,
    );

    let handle = unsafe { mun_gc_collect(driver.runtime, ptr::null_mut()) };
    assert_error_message_eq(handle, "Invalid argument: 'reclaimed' is null pointer.");
}

#[test]
fn test_gc_rooting() {
    let driver = TestDriver::new(
        r#"
        struct Foo;

        pub fn main() -> Foo { Foo }
    "#,
    );

    let type_name = CString::new("Foo").expect("Invalid type name");
    let mut has_type_info = false;
    let mut type_info = MaybeUninit::uninit();
    let handle = unsafe {
        mun_runtime_find_type_info_by_name(
            driver.runtime,
            type_name.as_ptr(),
            &mut has_type_info as *mut _,
            type_info.as_mut_ptr(),
        )
    };
    assert_eq!(handle.token(), 0);

    let type_info = unsafe { type_info.assume_init() };

    let mut obj = MaybeUninit::uninit();
    let handle = unsafe {
        mun_gc_alloc(
            driver.runtime,
            UnsafeTypeInfo::from(&type_info),
            obj.as_mut_ptr(),
        )
    };
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

use std::{ffi::CString, io::stderr, path::Path, ptr};

use mun_compiler::{Config, DisplayColor, Driver, PathOrInline, RelativePathBuf};

use crate::runtime::{mun_runtime_create, mun_runtime_destroy, Runtime, RuntimeOptions};

/// Combines a compiler and runtime in one. Use of the `TestDriver` allows for
/// quick testing of Mun constructs in the runtime with hot-reloading support.
pub(crate) struct TestDriver {
    _temp_dir: tempfile::TempDir,
    pub(crate) runtime: Runtime,
}

impl TestDriver {
    /// Constructs a new `TestDriver` from Mun source
    pub fn new(text: &str) -> Self {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config = Config {
            out_dir: Some(temp_dir.path().to_path_buf()),
            ..Config::default()
        };
        let input = PathOrInline::Inline {
            rel_path: RelativePathBuf::from("mod.mun"),
            contents: text.to_owned(),
        };
        let (mut driver, file_id) = Driver::with_file(config, input).unwrap();
        if driver
            .emit_diagnostics(&mut stderr(), DisplayColor::Disable)
            .unwrap()
        {
            panic!("compiler errors..")
        }
        let out_path = driver.assembly_output_path_from_file(file_id);
        driver.write_all_assemblies(false).unwrap();
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

fn make_runtime(lib_path: &Path) -> Runtime {
    let lib_path = lib_path.to_str().expect("Invalid lib path");
    let lib_path = CString::new(lib_path).unwrap();

    let mut handle = Runtime(ptr::null_mut());
    let error = unsafe {
        mun_runtime_create(
            lib_path.as_ptr(),
            RuntimeOptions::default(),
            &mut handle as *mut _,
        )
    };
    assert_eq!(error.0, ptr::null(), "Failed to create runtime");
    handle
}

/// A macro that generates tests for invalid runtime arguments
#[macro_export]
macro_rules! test_invalid_runtime {
    ($(
        $name:ident($($arg:expr),*)
    ),+) => {
        $(
            paste::item! {
                #[test]
                fn [<test_ $name _invalid_runtime>]() {
                    let runtime = Runtime(ptr::null_mut());
                    #[allow(clippy::macro_metavars_in_unsafe)]
                    let handle =
                        unsafe { [<mun_ $name>](runtime $(, $arg)*) };

                    let message = unsafe { std::ffi::CStr::from_ptr(handle.0) };
                    assert_eq!(
                        message.to_str().unwrap(),
                        "invalid argument 'runtime': null pointer"
                    );

                    unsafe { mun_error_destroy(handle) };
                }
            }
        )+
    };
}

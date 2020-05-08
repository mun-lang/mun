#![allow(dead_code, unused_macros)]

use mun_compiler::{Config, DisplayColor, Driver, FileId, PathOrInline, RelativePathBuf};
use mun_runtime::{IntoFunctionDefinition, Runtime, RuntimeBuilder};
use std::io::Cursor;
use std::{cell::RefCell, path::PathBuf, rc::Rc, thread::sleep, time::Duration};

/// Implements a compiler and runtime in one that can invoke functions. Use of the TestDriver
/// enables quick testing of Mun constructs in the runtime with hot-reloading support.
pub(crate) struct TestDriver {
    _temp_dir: tempfile::TempDir,
    out_path: PathBuf,
    file_id: FileId,
    driver: Driver,
    runtime: RuntimeOrBuilder,
}

enum RuntimeOrBuilder {
    Runtime(Rc<RefCell<Runtime>>),
    Builder(RuntimeBuilder),
    Pending,
}

impl RuntimeOrBuilder {
    pub fn spawn(&mut self) -> Result<(), failure::Error> {
        let previous = std::mem::replace(self, RuntimeOrBuilder::Pending);
        let runtime = match previous {
            RuntimeOrBuilder::Runtime(runtime) => runtime,
            RuntimeOrBuilder::Builder(builder) => builder.spawn()?,
            _ => unreachable!(),
        };
        std::mem::replace(self, RuntimeOrBuilder::Runtime(runtime));
        Ok(())
    }
}

impl TestDriver {
    /// Construct a new TestDriver from a single Mun source
    pub fn new(text: &str) -> Self {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config = Config {
            out_dir: Some(temp_dir.path().to_path_buf()),
            display_color: DisplayColor::Disable,
            ..Config::default()
        };
        let input = PathOrInline::Inline {
            rel_path: RelativePathBuf::from("main.mun"),
            contents: text.to_owned(),
        };
        let (mut driver, file_id) = Driver::with_file(config, input).unwrap();
        let mut compiler_errors: Vec<u8> = Vec::new();
        if driver
            .emit_diagnostics(&mut Cursor::new(&mut compiler_errors))
            .unwrap()
        {
            panic!(
                "compiler errors:\n{}",
                String::from_utf8(compiler_errors)
                    .expect("compiler errors are not UTF-8 formatted")
            )
        }
        let out_path = driver.write_assembly(file_id).unwrap();
        let builder = RuntimeBuilder::new(&out_path);
        TestDriver {
            _temp_dir: temp_dir,
            driver,
            out_path,
            file_id,
            runtime: RuntimeOrBuilder::Builder(builder),
        }
    }

    /// Spawns a `Runtime` from the `RuntimeBuilder`, if it hadn't already been spawned.
    pub fn spawn(&mut self) -> Result<(), failure::Error> {
        self.runtime.spawn().map(|_| ())
    }

    /// Updates the text of the Mun source and ensures that the generated assembly has been reloaded.
    pub fn update(&mut self, text: &str) {
        self.runtime_mut(); // Ensures that the runtime is spawned prior to the update
        self.driver.set_file_text(self.file_id, text);
        let mut compiler_errors: Vec<u8> = Vec::new();
        if self
            .driver
            .emit_diagnostics(&mut Cursor::new(&mut compiler_errors))
            .unwrap()
        {
            panic!(
                "compiler errors:\n{}",
                String::from_utf8(compiler_errors)
                    .expect("compiler errors are not UTF-8 formatted")
            )
        }
        let out_path = self.driver.write_assembly(self.file_id).unwrap();
        assert_eq!(
            &out_path, &self.out_path,
            "recompiling did not result in the same assembly"
        );
        let start_time = std::time::Instant::now();
        while !self.runtime_mut().borrow_mut().update() {
            let now = std::time::Instant::now();
            if now - start_time > std::time::Duration::from_secs(10) {
                panic!("runtime did not update after recompilation within 10secs");
            } else {
                sleep(Duration::from_millis(1));
            }
        }
    }

    /// Adds a custom user function to the dispatch table.
    pub fn insert_fn<S: AsRef<str>, F: IntoFunctionDefinition>(mut self, name: S, func: F) -> Self {
        self.runtime = match self.runtime {
            RuntimeOrBuilder::Builder(builder) => {
                RuntimeOrBuilder::Builder(builder.insert_fn(name, func))
            }
            _ => unreachable!(),
        };
        self
    }

    /// Returns the `Runtime` used by this instance
    pub fn runtime_mut(&mut self) -> &mut Rc<RefCell<Runtime>> {
        self.runtime.spawn().unwrap();
        match &mut self.runtime {
            RuntimeOrBuilder::Runtime(r) => r,
            _ => unreachable!(),
        }
    }
}

macro_rules! assert_invoke_eq {
    ($ExpectedType:ty, $ExpectedResult:expr, $Driver:expr, $($Arg:tt)+) => {
        let result: $ExpectedType = mun_runtime::invoke_fn!($Driver.runtime_mut(), $($Arg)*).unwrap();
        assert_eq!(result, $ExpectedResult, "{} == {:?}", stringify!(mun_runtime::invoke_fn!($Driver.runtime_mut(), $($Arg)*).unwrap()), $ExpectedResult);
    }
}

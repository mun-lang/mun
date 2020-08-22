#![allow(dead_code, unused_macros)]

use compiler::{Config, DisplayColor, Driver, FileId, PathOrInline, RelativePathBuf};
use mun_runtime::{IntoFunctionDefinition, Runtime, RuntimeBuilder};
use std::{
    cell::{Ref, RefCell},
    io::Cursor,
    path::PathBuf,
    rc::Rc,
    thread::sleep,
    time::Duration,
};

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
    pub fn spawn(&mut self) -> Result<(), anyhow::Error> {
        let previous = std::mem::replace(self, RuntimeOrBuilder::Pending);
        let runtime = match previous {
            RuntimeOrBuilder::Runtime(runtime) => runtime,
            RuntimeOrBuilder::Builder(builder) => builder.spawn()?,
            _ => unreachable!(),
        };
        *self = RuntimeOrBuilder::Runtime(runtime);
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
        let out_path = driver.assembly_output_path(file_id);
        driver.write_assembly(file_id, true).unwrap();
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
    pub fn spawn(&mut self) -> Result<(), anyhow::Error> {
        self.runtime.spawn().map(|_| ())
    }

    /// Updates the text of the Mun source and ensures that the generated assembly has been
    /// reloaded.
    ///
    /// A reference to the borrowed `runtime` is used as an argument to ensure that the runtime was
    /// spawned prior to calling update AND to allow moving of the existing borrow inside the update
    /// function. This obviates the necessity for `update` to use the `Runtime`.
    pub fn update(&mut self, runtime: Ref<'_, Runtime>, text: &str) {
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
        let out_path = self.driver.assembly_output_path(self.file_id);
        self.driver.write_assembly(self.file_id, true).unwrap();
        assert_eq!(
            &out_path, &self.out_path,
            "recompiling did not result in the same assembly"
        );
        let start_time = std::time::Instant::now();
        drop(runtime);
        while !self.runtime().borrow_mut().update() {
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
    pub fn runtime(&mut self) -> Rc<RefCell<Runtime>> {
        self.runtime.spawn().unwrap();
        match &mut self.runtime {
            RuntimeOrBuilder::Runtime(r) => r.clone(),
            _ => unreachable!(),
        }
    }
}

macro_rules! assert_invoke_eq {
    ($ExpectedType:ty, $ExpectedResult:expr, $Driver:expr, $($Arg:tt)+) => {
        {
            let runtime = $Driver.runtime();
            let runtime_ref = runtime.borrow();
            let result: $ExpectedType = mun_runtime::invoke_fn!(runtime_ref, $($Arg)*).unwrap();
            assert_eq!(
                result, $ExpectedResult, "{} == {:?}",
                stringify!(mun_runtime::invoke_fn!(runtime_ref, $($Arg)*).unwrap()),
                $ExpectedResult
            );
        }
    }
}

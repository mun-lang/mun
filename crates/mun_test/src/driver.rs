use compiler::{Config, DisplayColor, Driver, FileId, PathOrInline, RelativePathBuf};
use runtime::{Runtime, RuntimeBuilder};
use std::{
    cell::{Ref, RefCell},
    io::Cursor,
    path::{Path, PathBuf},
    rc::Rc,
    thread::sleep,
    time::{Duration, Instant},
};

/// Implements a compiler that generates and temporarily stores a `*.munlib` library
/// corresponding to a single source file.
pub struct CompileTestDriver {
    _temp_dir: tempfile::TempDir,
    out_path: PathBuf,
    file_id: FileId,
    driver: Driver,
}

impl CompileTestDriver {
    /// Constructs a new `CompileTestDriver` from a single Mun source.
    pub fn new(text: &str) -> Self {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config = Config {
            out_dir: Some(temp_dir.path().to_path_buf()),
            display_color: DisplayColor::Disable,
            ..Config::default()
        };
        let input = PathOrInline::Inline {
            rel_path: RelativePathBuf::from("mod.mun"),
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

        driver.write_all_assemblies(true).unwrap();
        let out_path = driver.assembly_output_path_from_file(file_id);

        CompileTestDriver {
            _temp_dir: temp_dir,
            driver,
            out_path,
            file_id,
        }
    }

    /// Updates the text of the Mun source and ensures that the generated assembly has been
    /// recompiled.
    pub fn update(&mut self, text: &str) {
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
        self.driver.write_all_assemblies(true).unwrap();
        let out_path = self.driver.assembly_output_path_from_file(self.file_id);
        assert_eq!(
            &out_path, &self.out_path,
            "recompiling did not result in the same assembly"
        );
    }

    /// Returns the path to the generated `*.munlib` library.
    pub fn lib_path(&self) -> &Path {
        &self.out_path
    }
}

impl std::fmt::Debug for CompileTestDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompilerTestDriver")
            .field("_temp_dir", &self._temp_dir)
            .field("out_path", &self.out_path)
            .field("file_id", &self.file_id)
            .finish()
    }
}

/// Implements a compiler that generates, temporarily stores, and hot reloads a
/// `*.munlib` library corresponding to a single source file.
///
/// This allows testing of Mun constructs that depend on hot-reloading.
pub struct CompileAndRunTestDriver {
    driver: CompileTestDriver,
    runtime: Rc<RefCell<Runtime>>,
}

impl std::fmt::Debug for CompileAndRunTestDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompileAndRunTestDriver")
            .field("driver", &self.driver)
            .finish()
    }
}

impl CompileAndRunTestDriver {
    /// Constructs a `CompileAndRunTestDriver` from a single Mun source file and a `config_fn` that
    /// allows modification of a [`RuntimeBuilder`].
    pub fn new(
        text: &str,
        config_fn: impl FnOnce(RuntimeBuilder) -> RuntimeBuilder,
    ) -> Result<Self, anyhow::Error> {
        let driver = CompileTestDriver::new(text);
        let builder = RuntimeBuilder::new(driver.lib_path());
        let runtime = config_fn(builder).spawn()?;

        Ok(Self { driver, runtime })
    }

    /// Updates the text of the Mun source and ensures that the generated assembly has been
    /// reloaded.
    ///
    /// A reference to the borrowed `runtime` is used as an argument to allow moving of the
    /// existing borrow inside the update function. This obviates the necessity for `update` to use
    /// the `Runtime`.
    pub fn update(&mut self, runtime: Ref<'_, Runtime>, text: &str) {
        self.driver.update(text);

        let start_time = Instant::now();
        drop(runtime);
        while !self.runtime().borrow_mut().update() {
            let now = Instant::now();
            if now - start_time > Duration::from_secs(10) {
                panic!("runtime did not update after recompilation within 10 seconds");
            } else {
                sleep(Duration::from_millis(1));
            }
        }
    }

    /// Returns the `Runtime` used by the driver.
    pub fn runtime(&self) -> Rc<RefCell<Runtime>> {
        self.runtime.clone()
    }
}

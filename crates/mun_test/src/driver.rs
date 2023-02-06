use mun_compiler::{Config, DisplayColor, Driver, PathOrInline, RelativePathBuf};
use mun_runtime::{Runtime, RuntimeBuilder};
use std::{
    path::{Path, PathBuf},
    thread::sleep,
    time::{Duration, Instant},
};

/// Implements a compiler that generates and temporarily stores a `*.munlib` library
/// corresponding to a single source file.
pub struct CompileTestDriver {
    _temp_output_dir: tempfile::TempDir,
    _temp_workspace: Option<tempfile::TempDir>,
    out_path: PathBuf,
    driver: Driver,
}

impl CompileTestDriver {
    /// Constructs a new `CompilerTestDriver` from a fixture that describes an entire mun project.
    /// So it file structure should look something like this:
    /// ```text
    /// mun.toml
    /// src/
    ///    mod.mun
    /// ```
    pub fn from_fixture(text: &str) -> Self {
        let temp_output_dir = tempfile::TempDir::new().unwrap();
        let config = Config {
            out_dir: Some(temp_output_dir.path().to_path_buf()),
            ..Config::default()
        };

        // Write the contents of the fixture to a temporary directory
        let temp_source_dir = tempfile::TempDir::new().unwrap();
        for entry in mun_hir::fixture::Fixture::parse(text) {
            let path = entry.relative_path.to_path(temp_source_dir.path());
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, entry.text).unwrap();
        }

        // Initialize the driver from the fixture content
        let (_, mut driver) =
            Driver::with_package_path(temp_source_dir.path().join("mun.toml"), config).unwrap();
        if let Some(compiler_errors) = driver
            .emit_diagnostics_to_string(DisplayColor::Disable)
            .expect("could not create diagnostics")
        {
            panic!("compiler errors:\n{compiler_errors}")
        }

        driver.write_all_assemblies(true).unwrap();
        let out_path = temp_output_dir.path().join("mod.munlib");

        CompileTestDriver {
            _temp_output_dir: temp_output_dir,
            _temp_workspace: Some(temp_source_dir),
            driver,
            out_path,
        }
    }

    /// Constructs a new `CompileTestDriver` from a single Mun source.
    pub fn from_file(text: &str) -> Self {
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
        if let Some(compiler_errors) = driver
            .emit_diagnostics_to_string(DisplayColor::Disable)
            .expect("could not generate compiler diagnostics")
        {
            panic!("compiler errors:\n{compiler_errors}")
        }

        driver.write_all_assemblies(true).unwrap();
        let out_path = driver.assembly_output_path_from_file(file_id);

        CompileTestDriver {
            _temp_output_dir: temp_dir,
            _temp_workspace: None,
            driver,
            out_path,
        }
    }

    /// Updates the text of the Mun source and ensures that the generated assembly has been
    /// recompiled.
    pub fn update(&mut self, path: impl AsRef<mun_paths::RelativePath>, text: &str) {
        self.driver.set_file_text(path, text).unwrap();

        let compiler_errors = self
            .driver
            .emit_diagnostics_to_string(DisplayColor::Disable)
            .expect("error creating diagnostics");
        if let Some(compiler_errors) = compiler_errors {
            panic!("compiler errors:\n{compiler_errors}")
        }
        self.driver.write_all_assemblies(true).unwrap();
    }

    /// Returns the path to the generated `*.munlib` library.
    pub fn lib_path(&self) -> &Path {
        &self.out_path
    }
}

impl std::fmt::Debug for CompileTestDriver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompilerTestDriver")
            .field("_temp_dir", &self._temp_output_dir)
            .field("out_path", &self.out_path)
            .finish()
    }
}

/// Implements a compiler that generates, temporarily stores, and hot reloads a
/// `*.munlib` library corresponding to a single source file.
///
/// This allows testing of Mun constructs that depend on hot-reloading.
pub struct CompileAndRunTestDriver {
    driver: CompileTestDriver,

    /// The runtime created by this instance.
    pub runtime: Runtime,
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
    pub fn from_fixture(
        fixture: &str,
        config_fn: impl FnOnce(RuntimeBuilder) -> RuntimeBuilder,
    ) -> Result<Self, anyhow::Error> {
        let driver = CompileTestDriver::from_fixture(fixture);
        let builder = Runtime::builder(driver.lib_path());

        // Safety: We compiled the library ourselves, therefor loading the munlib is safe.
        let build = config_fn(builder);
        let runtime = unsafe { build.finish() }?;

        Ok(Self { driver, runtime })
    }

    /// Constructs a `CompileAndRunTestDriver` from a single Mun source file and a `config_fn` that
    /// allows modification of a [`RuntimeBuilder`].
    pub fn new(
        text: &str,
        config_fn: impl FnOnce(RuntimeBuilder) -> RuntimeBuilder,
    ) -> Result<Self, anyhow::Error> {
        let driver = CompileTestDriver::from_file(text);
        let builder = Runtime::builder(driver.lib_path());

        // Safety: We compiled the library ourselves, therefor loading the munlib is safe.
        let build = config_fn(builder);
        let runtime = unsafe { build.finish() }?;

        Ok(Self { driver, runtime })
    }

    /// Updates the text of the Mun source and ensures that the generated assembly has been
    /// reloaded.
    ///
    /// A reference to the borrowed `runtime` is used as an argument to allow moving of the
    /// existing borrow inside the update function. This obviates the necessity for `update` to use
    /// the `Runtime`.
    pub fn update(&mut self, path: impl AsRef<mun_paths::RelativePath>, text: &str) {
        self.driver.update(path, text);

        let start_time = Instant::now();

        // Safety: We compiled the library ourselves, therefor updating the runtime is safe.
        while !unsafe { self.runtime.update() } {
            let now = Instant::now();
            if now - start_time > Duration::from_secs(10) {
                panic!("runtime did not update after recompilation within 10 seconds");
            } else {
                sleep(Duration::from_millis(1));
            }
        }
    }
}

//! Code to perform tests on Mun code.

use mun_compiler::{Config, DisplayColor, PathOrInline, RelativePathBuf};
use mun_runtime::RuntimeBuilder;

/// The type of test to create
#[derive(Copy, Clone)]
pub enum TestMode {
    /// Compile the code to ensure it compiles and run the `main` function which should not panic
    CompileAndRun,

    /// Only compile the code to ensure its valid Mun code
    Compile,

    /// Compile the code but it should fail to compile
    ShouldNotCompile,
}

impl TestMode {
    /// Returns true if the Mun code of the test should be compiled
    fn should_compile(self) -> bool {
        matches!(self, TestMode::CompileAndRun | TestMode::Compile)
    }

    /// Returns true if the Mun code should be invoked
    fn should_run(self) -> bool {
        matches!(self, TestMode::CompileAndRun)
    }
}

/// Run a Mun test with the specified `code`.
pub fn run_test(code: &str, mode: TestMode) {
    // Construct a temporary path to store the output files
    let out_dir = tempdir::TempDir::new("mun_test_")
        .expect("could not create temporary directory for test output");

    // Construct a driver to compile the code with
    let (mut driver, file_id) = mun_compiler::Driver::with_file(
        Config {
            out_dir: Some(out_dir.path().to_path_buf()),
            ..Config::default()
        },
        PathOrInline::Inline {
            rel_path: RelativePathBuf::from("mod.mun"),
            contents: code.to_owned(),
        },
    )
    .expect("unable to create driver from test input");

    // Check if the code compiles (and whether thats ok)
    let compiler_errors = driver
        .emit_diagnostics_to_string(DisplayColor::Auto)
        .expect("error emitting errors");
    match (compiler_errors, mode.should_compile()) {
        (Some(errors), true) => {
            panic!("code contains compiler errors:\n{}", errors);
        }
        (None, false) => {
            panic!("Code that should have caused the error compiled successfully");
        }
        _ => (),
    };

    if !mode.should_run() {
        return;
    }

    // Write the library to the output so we can run it
    driver
        .write_all_assemblies(true)
        .expect("error emitting assemblies");

    // Create a runtime
    let assembly_path = driver.assembly_output_path_from_file(file_id);
    let runtime = RuntimeBuilder::new(assembly_path)
        .spawn()
        .expect("error creating runtime for test assembly");

    // Find the main function
    if runtime.borrow().get_function_definition("main").is_none() {
        panic!("Could not find `main` function");
    }

    // Call the main function
    let _: () = runtime
        .borrow_mut()
        .invoke("main", ())
        .expect("error calling main function");
}

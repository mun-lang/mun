use std::path::{PathBuf};
use mun_compiler::CompilerOptions;
use crate::{RuntimeBuilder, MunRuntime};

struct CompileResult {
    _temp_dir: tempfile::TempDir,
    result: PathBuf
}

impl CompileResult {
    /// Construct a runtime from the compilation result that can be used to execute the compiled
    /// files.
    pub fn new_runtime(&self) -> MunRuntime {
        RuntimeBuilder::new(&self.result).spawn().unwrap()
    }
}

/// Compiles the given mun and returns a `CompileResult` that can be used to execute it.
fn compile(text: &str) -> CompileResult {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let options = CompilerOptions {
        out_dir: Some(temp_dir.path().to_path_buf()),
        .. CompilerOptions::with_file(text)
    };
    let result = mun_compiler::main(&options).unwrap().unwrap();
    CompileResult {
        _temp_dir: temp_dir,
        result
    }
}

#[test]
fn compile_and_run() {
    let compile_result = compile(r"
        fn main() {}
    ");
    let mut runtime = compile_result.new_runtime();
    let _result: () = invoke_fn!(runtime, "main");
}

#[test]
fn return_value() {
    let compile_result = compile(r"
        fn main():int { 3 }
    ");
    let mut runtime = compile_result.new_runtime();
    let result: i64 = invoke_fn!(runtime, "main");
    assert_eq!(result, 3);
}

#[test]
fn arguments() {
    let compile_result = compile(r"
        fn main(a:int, b:int):int { a+b }
    ");
    let mut runtime = compile_result.new_runtime();
    let a:i64 = 52;
    let b:i64 = 746;
    let result: i64 = invoke_fn!(runtime, "main", a, b);
    assert_eq!(result, a+b);
}
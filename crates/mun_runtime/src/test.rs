use crate::{Runtime, RuntimeBuilder};
use mun_compiler::CompilerOptions;
use std::path::PathBuf;

struct CompileResult {
    _temp_dir: tempfile::TempDir,
    result: PathBuf,
}

impl CompileResult {
    /// Construct a runtime from the compilation result that can be used to execute the compiled
    /// files.
    pub fn new_runtime(&self) -> Runtime {
        RuntimeBuilder::new(&self.result).spawn().unwrap()
    }
}

/// Compiles the given mun and returns a `CompileResult` that can be used to execute it.
fn compile(text: &str) -> CompileResult {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let options = CompilerOptions {
        out_dir: Some(temp_dir.path().to_path_buf()),
        ..CompilerOptions::with_file(text)
    };
    let result = mun_compiler::main(&options).unwrap().unwrap();
    CompileResult {
        _temp_dir: temp_dir,
        result,
    }
}

#[test]
fn compile_and_run() {
    let compile_result = compile(
        r"
        fn main() {}
    ",
    );
    let mut runtime = compile_result.new_runtime();
    let _result: () = invoke_fn!(runtime, "main").unwrap();
}

#[test]
fn return_value() {
    let compile_result = compile(
        r"
        fn main():int { 3 }
    ",
    );
    let mut runtime = compile_result.new_runtime();
    let result: i64 = invoke_fn!(runtime, "main").unwrap();
    assert_eq!(result, 3);
}

#[test]
fn arguments() {
    let compile_result = compile(
        r"
        fn main(a:int, b:int):int { a+b }
    ",
    );
    let mut runtime = compile_result.new_runtime();
    let a: i64 = 52;
    let b: i64 = 746;
    let result: i64 = invoke_fn!(runtime, "main", a, b).unwrap();
    assert_eq!(result, a + b);
}

#[test]
fn dispatch_table() {
    let compile_result = compile(
        r"
        fn add(a:int, b:int):int { a+b }
        fn main(a:int, b:int):int { add(a,b) }
    ",
    );
    let mut runtime = compile_result.new_runtime();

    let a: i64 = 52;
    let b: i64 = 746;
    let result: i64 = invoke_fn!(runtime, "main", a, b).unwrap();
    assert_eq!(result, a + b);

    let a: i64 = 6274;
    let b: i64 = 72;
    let result: i64 = invoke_fn!(runtime, "add", a, b).unwrap();
    assert_eq!(result, a + b);
}

#[test]
fn booleans() {
    let compile_result = compile(
        r#"
        fn equal(a:int, b:int):bool                 { a==b }
        fn equalf(a:float, b:float):bool            { a==b }
        fn not_equal(a:int, b:int):bool             { a!=b }
        fn not_equalf(a:float, b:float):bool        { a!=b }
        fn less(a:int, b:int):bool                  { a<b }
        fn lessf(a:float, b:float):bool             { a<b }
        fn greater(a:int, b:int):bool               { a>b }
        fn greaterf(a:float, b:float):bool          { a>b }
        fn less_equal(a:int, b:int):bool            { a<=b }
        fn lessf_equal(a:float, b:float):bool       { a<=b }
        fn greater_equal(a:int, b:int):bool         { a>=b }
        fn greaterf_equal(a:float, b:float):bool    { a>=b }
    "#,
    );
    let mut runtime = compile_result.new_runtime();
    assert_eq!(
        Runtime::invoke_fn2::<i64, i64, bool>(&mut runtime, "equal", 52, 764).unwrap(),
        false
    );
    assert_eq!(
        Runtime::invoke_fn2::<i64, i64, bool>(&mut runtime, "equal", 64, 64).unwrap(),
        true
    );
    assert_eq!(
        Runtime::invoke_fn2::<f64, f64, bool>(&mut runtime, "equalf", 123.0, 123.0).unwrap(),
        true
    );
    assert_eq!(
        Runtime::invoke_fn2::<f64, f64, bool>(&mut runtime, "equalf", 123.0, 234.0).unwrap(),
        false
    );
    assert_eq!(
        Runtime::invoke_fn2::<i64, i64, bool>(&mut runtime, "not_equal", 52, 764).unwrap(),
        true
    );
    assert_eq!(
        Runtime::invoke_fn2::<i64, i64, bool>(&mut runtime, "not_equal", 64, 64).unwrap(),
        false
    );
    assert_eq!(
        Runtime::invoke_fn2::<f64, f64, bool>(&mut runtime, "not_equalf", 123.0, 123.0).unwrap(),
        false
    );
    assert_eq!(
        Runtime::invoke_fn2::<f64, f64, bool>(&mut runtime, "not_equalf", 123.0, 234.0).unwrap(),
        true
    );
    assert_eq!(
        Runtime::invoke_fn2::<i64, i64, bool>(&mut runtime, "less", 52, 764).unwrap(),
        true
    );
    assert_eq!(
        Runtime::invoke_fn2::<i64, i64, bool>(&mut runtime, "less", 64, 64).unwrap(),
        false
    );
    assert_eq!(
        Runtime::invoke_fn2::<f64, f64, bool>(&mut runtime, "lessf", 123.0, 123.0).unwrap(),
        false
    );
    assert_eq!(
        Runtime::invoke_fn2::<f64, f64, bool>(&mut runtime, "lessf", 123.0, 234.0).unwrap(),
        true
    );
    assert_eq!(
        Runtime::invoke_fn2::<i64, i64, bool>(&mut runtime, "greater", 52, 764).unwrap(),
        false
    );
    assert_eq!(
        Runtime::invoke_fn2::<i64, i64, bool>(&mut runtime, "greater", 64, 64).unwrap(),
        false
    );
    assert_eq!(
        Runtime::invoke_fn2::<f64, f64, bool>(&mut runtime, "greaterf", 123.0, 123.0).unwrap(),
        false
    );
    assert_eq!(
        Runtime::invoke_fn2::<f64, f64, bool>(&mut runtime, "greaterf", 123.0, 234.0).unwrap(),
        false
    );
    assert_eq!(
        Runtime::invoke_fn2::<i64, i64, bool>(&mut runtime, "less_equal", 52, 764).unwrap(),
        true
    );
    assert_eq!(
        Runtime::invoke_fn2::<i64, i64, bool>(&mut runtime, "less_equal", 64, 64).unwrap(),
        true
    );
    assert_eq!(
        Runtime::invoke_fn2::<f64, f64, bool>(&mut runtime, "lessf_equal", 123.0, 123.0).unwrap(),
        true
    );
    assert_eq!(
        Runtime::invoke_fn2::<f64, f64, bool>(&mut runtime, "lessf_equal", 123.0, 234.0).unwrap(),
        true
    );
    assert_eq!(
        Runtime::invoke_fn2::<i64, i64, bool>(&mut runtime, "greater_equal", 52, 764).unwrap(),
        false
    );
    assert_eq!(
        Runtime::invoke_fn2::<i64, i64, bool>(&mut runtime, "greater_equal", 64, 64).unwrap(),
        true
    );
    assert_eq!(
        Runtime::invoke_fn2::<f64, f64, bool>(&mut runtime, "greaterf_equal", 123.0, 123.0)
            .unwrap(),
        true
    );
    assert_eq!(
        Runtime::invoke_fn2::<f64, f64, bool>(&mut runtime, "greaterf_equal", 123.0, 234.0)
            .unwrap(),
        false
    );
}

#[test]
fn fibonacci() {
    let compile_result = compile(
        r#"
    fn fibonacci(n:int):int {
        if n <= 1 {
            n
        } else {
            fibonacci(n-1) + fibonacci(n-2)
        }
    }
    "#,
    );
    let mut runtime = compile_result.new_runtime();
    assert_eq!(
        Runtime::invoke_fn1::<i64, i64>(&mut runtime, "fibonacci", 5).unwrap(),
        5
    );
    assert_eq!(
        Runtime::invoke_fn1::<i64, i64>(&mut runtime, "fibonacci", 11).unwrap(),
        89
    );
    assert_eq!(
        Runtime::invoke_fn1::<i64, i64>(&mut runtime, "fibonacci", 16).unwrap(),
        987
    );
}

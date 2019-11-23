use crate::{Runtime, RuntimeBuilder};
use mun_compiler::{ColorChoice, Config, Driver, FileId, PathOrInline, RelativePathBuf};
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

/// Implements a compiler and runtime in one that can invoke functions. Use of the TestDriver
/// enables quick testing of Mun constructs in the runtime with hot-reloading support.
struct TestDriver {
    _temp_dir: tempfile::TempDir,
    out_path: PathBuf,
    file_id: FileId,
    driver: Driver,
    runtime: Runtime,
}

impl TestDriver {
    /// Construct a new TestDriver from a single Mun source
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
        let (driver, file_id) = Driver::with_file(config, input).unwrap();
        let mut err_stream = mun_compiler::StandardStream::stderr(ColorChoice::Auto);
        if driver.emit_diagnostics(&mut err_stream).unwrap() {
            panic!("compiler errors..")
        }
        let out_path = driver.write_assembly(file_id).unwrap().unwrap();
        let runtime = RuntimeBuilder::new(&out_path).spawn().unwrap();
        TestDriver {
            _temp_dir: temp_dir,
            driver,
            out_path,
            file_id,
            runtime,
        }
    }

    /// Updates the text of the Mun source and ensures that the generated assembly has been reloaded.
    fn update(&mut self, text: &str) {
        self.driver.set_file_text(self.file_id, text);
        let out_path = self.driver.write_assembly(self.file_id).unwrap().unwrap();
        assert_eq!(
            &out_path, &self.out_path,
            "recompiling did not result in the same assembly"
        );
        let start_time = std::time::Instant::now();
        while !self.runtime.update() {
            let now = std::time::Instant::now();
            if now - start_time > std::time::Duration::from_secs(10) {
                panic!("runtime did not update after recompilation within 10secs");
            } else {
                sleep(Duration::from_millis(1));
            }
        }
    }

    /// Returns the `Runtime` used by this instance
    fn runtime_mut(&mut self) -> &mut Runtime {
        &mut self.runtime
    }
}

macro_rules! assert_invoke_eq {
    ($ExpectedType:ty, $ExpectedResult:expr, $Driver:expr, $($Arg:tt)+) => {
        let result: $ExpectedType = invoke_fn!($Driver.runtime_mut(), $($Arg)*).unwrap();
        assert_eq!(result, $ExpectedResult, "{} == {:?}", stringify!(invoke_fn!($Driver.runtime_mut(), $($Arg)*).unwrap()), $ExpectedResult);
    }
}

#[test]
fn compile_and_run() {
    let mut driver = TestDriver::new(
        r"
        fn main() {}
    ",
    );
    assert_invoke_eq!((), (), driver, "main");
}

#[test]
fn return_value() {
    let mut driver = TestDriver::new(
        r"
        fn main():int { 3 }
    ",
    );
    assert_invoke_eq!(i64, 3, driver, "main");
}

#[test]
fn arguments() {
    let mut driver = TestDriver::new(
        r"
        fn main(a:int, b:int):int { a+b }
    ",
    );
    let a: i64 = 52;
    let b: i64 = 746;
    assert_invoke_eq!(i64, a + b, driver, "main", a, b);
}

#[test]
fn dispatch_table() {
    let mut driver = TestDriver::new(
        r"
        fn add(a:int, b:int):int { a+b }
        fn main(a:int, b:int):int { add(a,b) }
    ",
    );

    let a: i64 = 52;
    let b: i64 = 746;
    assert_invoke_eq!(i64, a + b, driver, "main", a, b);

    let a: i64 = 6274;
    let b: i64 = 72;
    assert_invoke_eq!(i64, a + b, driver, "add", a, b);
}

#[test]
fn booleans() {
    let mut driver = TestDriver::new(
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
        fn less_equalf(a:float, b:float):bool       { a<=b }
        fn greater_equal(a:int, b:int):bool         { a>=b }
        fn greater_equalf(a:float, b:float):bool    { a>=b }
    "#,
    );
    assert_invoke_eq!(bool, false, driver, "equal", 52i64, 764i64);
    assert_invoke_eq!(bool, true, driver, "equal", 64i64, 64i64);
    assert_invoke_eq!(bool, false, driver, "equalf", 52f64, 764f64);
    assert_invoke_eq!(bool, true, driver, "equalf", 64f64, 64f64);
    assert_invoke_eq!(bool, true, driver, "not_equal", 52i64, 764i64);
    assert_invoke_eq!(bool, false, driver, "not_equal", 64i64, 64i64);
    assert_invoke_eq!(bool, true, driver, "not_equalf", 52f64, 764f64);
    assert_invoke_eq!(bool, false, driver, "not_equalf", 64f64, 64f64);
    assert_invoke_eq!(bool, true, driver, "less", 52i64, 764i64);
    assert_invoke_eq!(bool, false, driver, "less", 64i64, 64i64);
    assert_invoke_eq!(bool, true, driver, "lessf", 52f64, 764f64);
    assert_invoke_eq!(bool, false, driver, "lessf", 64f64, 64f64);
    assert_invoke_eq!(bool, false, driver, "greater", 52i64, 764i64);
    assert_invoke_eq!(bool, false, driver, "greater", 64i64, 64i64);
    assert_invoke_eq!(bool, false, driver, "greaterf", 52f64, 764f64);
    assert_invoke_eq!(bool, false, driver, "greaterf", 64f64, 64f64);
    assert_invoke_eq!(bool, true, driver, "less_equal", 52i64, 764i64);
    assert_invoke_eq!(bool, true, driver, "less_equal", 64i64, 64i64);
    assert_invoke_eq!(bool, true, driver, "less_equalf", 52f64, 764f64);
    assert_invoke_eq!(bool, true, driver, "less_equalf", 64f64, 64f64);
    assert_invoke_eq!(bool, false, driver, "greater_equal", 52i64, 764i64);
    assert_invoke_eq!(bool, true, driver, "greater_equal", 64i64, 64i64);
    assert_invoke_eq!(bool, false, driver, "greater_equalf", 52f64, 764f64);
    assert_invoke_eq!(bool, true, driver, "greater_equalf", 64f64, 64f64);
}

#[test]
fn fibonacci() {
    let mut driver = TestDriver::new(
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

    assert_invoke_eq!(i64, 5, driver, "fibonacci", 5i64);
    assert_invoke_eq!(i64, 89, driver, "fibonacci", 11i64);
    assert_invoke_eq!(i64, 987, driver, "fibonacci", 16i64);
}

#[test]
fn fibonacci_loop() {
    let mut driver = TestDriver::new(
        r#"
    fn fibonacci(n:int):int {
        let a = 0;
        let b = 1;
        let i = 1;
        loop {
            if i > n {
                return a
            }
            let sum = a + b;
            a = b;
            b = sum;
            i += 1;
        }
    }
    "#,
    );

    assert_invoke_eq!(i64, 5, driver, "fibonacci", 5i64);
    assert_invoke_eq!(i64, 89, driver, "fibonacci", 11i64);
    assert_invoke_eq!(i64, 987, driver, "fibonacci", 16i64);
    assert_invoke_eq!(i64, 46368, driver, "fibonacci", 24i64);
}

#[test]
fn fibonacci_loop_break() {
    let mut driver = TestDriver::new(
        r#"
    fn fibonacci(n:int):int {
        let a = 0;
        let b = 1;
        let i = 1;
        loop {
            if i > n {
                break a;
            }
            let sum = a + b;
            a = b;
            b = sum;
            i += 1;
        }
    }
    "#,
    );

    assert_invoke_eq!(i64, 5, driver, "fibonacci", 5i64);
    assert_invoke_eq!(i64, 89, driver, "fibonacci", 11i64);
    assert_invoke_eq!(i64, 987, driver, "fibonacci", 16i64);
    assert_invoke_eq!(i64, 46368, driver, "fibonacci", 24i64);
}

#[test]
fn true_is_true() {
    let mut driver = TestDriver::new(
        r#"
    fn test_true():bool {
        true
    }

    fn test_false():bool {
        false
    }
    "#,
    );
    assert_invoke_eq!(bool, true, driver, "test_true");
    assert_invoke_eq!(bool, false, driver, "test_false");
}

#[test]
fn hotreloadable() {
    let mut driver = TestDriver::new(
        r"
    fn main():int { 5 }
    ",
    );
    assert_invoke_eq!(i64, 5, driver, "main");
    driver.update(
        r"
    fn main():int { 10 }
    ",
    );
    assert_invoke_eq!(i64, 10, driver, "main");
}

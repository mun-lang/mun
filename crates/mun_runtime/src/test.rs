use crate::{Runtime, RuntimeBuilder};
use mun_abi::Reflection;
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

    /// Calls the function with the specified name
    fn invoke0<T: Reflection>(&mut self, fn_name: &str) -> T {
        Runtime::invoke_fn0::<T>(&mut self.runtime, fn_name).unwrap()
    }

    /// Calls the function with the specified name passing a single argument
    fn invoke1<A: Reflection, T: Reflection>(&mut self, fn_name: &str, arg0: A) -> T {
        Runtime::invoke_fn1::<A, T>(&mut self.runtime, fn_name, arg0).unwrap()
    }

    /// Calls the function with the specified name passing two arguments
    fn invoke2<A: Reflection, B: Reflection, T: Reflection>(
        &mut self,
        fn_name: &str,
        arg0: A,
        arg1: B,
    ) -> T {
        Runtime::invoke_fn2::<A, B, T>(&mut self.runtime, fn_name, arg0, arg1).unwrap()
    }
}

#[test]
fn compile_and_run() {
    let mut driver = TestDriver::new(
        r"
        fn main() {}
    ",
    );
    let _result: () = driver.invoke0("main");
}

#[test]
fn return_value() {
    let mut driver = TestDriver::new(
        r"
        fn main():int { 3 }
    ",
    );
    let result: i64 = driver.invoke0("main");
    assert_eq!(result, 3);
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
    let result: i64 = driver.invoke2("main", a, b);
    assert_eq!(result, a + b);
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
    assert_eq!(driver.invoke2::<i64, i64, i64>("main", a, b), a + b);

    let a: i64 = 6274;
    let b: i64 = 72;
    assert_eq!(driver.invoke2::<i64, i64, i64>("add", a, b), a + b);
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
        fn lessf_equal(a:float, b:float):bool       { a<=b }
        fn greater_equal(a:int, b:int):bool         { a>=b }
        fn greaterf_equal(a:float, b:float):bool    { a>=b }
    "#,
    );
    assert_eq!(driver.invoke2::<_, _, bool>("equal", 52i64, 764i64), false);
    assert_eq!(driver.invoke2::<_, _, bool>("equal", 64i64, 64i64), true);
    assert_eq!(driver.invoke2::<_, _, bool>("equalf", 52f64, 764f64), false);
    assert_eq!(driver.invoke2::<_, _, bool>("equalf", 64f64, 64f64), true);
    assert_eq!(
        driver.invoke2::<_, _, bool>("not_equal", 52i64, 764i64),
        true
    );
    assert_eq!(
        driver.invoke2::<_, _, bool>("not_equal", 64i64, 64i64),
        false
    );
    assert_eq!(
        driver.invoke2::<_, _, bool>("not_equalf", 52f64, 764f64),
        true
    );
    assert_eq!(
        driver.invoke2::<_, _, bool>("not_equalf", 64f64, 64f64),
        false
    );
    assert_eq!(driver.invoke2::<_, _, bool>("less", 52i64, 764i64), true);
    assert_eq!(driver.invoke2::<_, _, bool>("less", 64i64, 64i64), false);
    assert_eq!(driver.invoke2::<_, _, bool>("lessf", 52f64, 764f64), true);
    assert_eq!(driver.invoke2::<_, _, bool>("lessf", 64f64, 64f64), false);
    assert_eq!(
        driver.invoke2::<_, _, bool>("greater", 52i64, 764i64),
        false
    );
    assert_eq!(driver.invoke2::<_, _, bool>("greater", 64i64, 64i64), false);
    assert_eq!(
        driver.invoke2::<_, _, bool>("greaterf", 52f64, 764f64),
        false
    );
    assert_eq!(
        driver.invoke2::<_, _, bool>("greaterf", 64f64, 64f64),
        false
    );
    assert_eq!(
        driver.invoke2::<_, _, bool>("less_equal", 52i64, 764i64),
        true
    );
    assert_eq!(
        driver.invoke2::<_, _, bool>("less_equal", 64i64, 64i64),
        true
    );
    assert_eq!(
        driver.invoke2::<_, _, bool>("lessf_equal", 52f64, 764f64),
        true
    );
    assert_eq!(
        driver.invoke2::<_, _, bool>("lessf_equal", 64f64, 64f64),
        true
    );
    assert_eq!(
        driver.invoke2::<_, _, bool>("greater_equal", 52i64, 764i64),
        false
    );
    assert_eq!(
        driver.invoke2::<_, _, bool>("greater_equal", 64i64, 64i64),
        true
    );
    assert_eq!(
        driver.invoke2::<_, _, bool>("greaterf_equal", 52f64, 764f64),
        false
    );
    assert_eq!(
        driver.invoke2::<_, _, bool>("greaterf_equal", 64f64, 64f64),
        true
    );
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

    assert_eq!(driver.invoke1::<i64, i64>("fibonacci", 5i64), 5);
    assert_eq!(driver.invoke1::<i64, i64>("fibonacci", 11i64), 89);
    assert_eq!(driver.invoke1::<i64, i64>("fibonacci", 16i64), 987);
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
    assert_eq!(driver.invoke0::<bool>("test_true"), true);
    assert_eq!(driver.invoke0::<bool>("test_false"), false);
}

#[test]
fn hotreloadable() {
    let mut driver = TestDriver::new(
        r"
    fn main():int { 5 }
    ",
    );
    assert_eq!(driver.invoke0::<i64>("main"), 5);
    driver.update(
        r"
    fn main():int { 10 }
    ",
    );
    assert_eq!(driver.invoke0::<i64>("main"), 10);
}

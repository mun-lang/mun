use crate::{Runtime, RuntimeBuilder, Struct};
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
fn fibonacci_while() {
    let mut driver = TestDriver::new(
        r#"
    fn fibonacci(n:int):int {
        let a = 0;
        let b = 1;
        let i = 1;
        while i <= n {
            let sum = a + b;
            a = b;
            b = sum;
            i += 1;
        }
        a
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

#[test]
fn compiler_valid_utf8() {
    use std::ffi::CStr;
    use std::slice;

    let driver = TestDriver::new(
        r#"
    struct Foo {
        a: int,
    }

    fn foo(n:Foo):bool { false }
    "#,
    );

    let foo_func = driver.runtime.get_function_info("foo").unwrap();
    assert_eq!(
        unsafe { CStr::from_ptr(foo_func.signature.name) }
            .to_str()
            .is_ok(),
        true
    );

    for arg_type in foo_func.signature.arg_types() {
        assert_eq!(
            unsafe { CStr::from_ptr(arg_type.name) }.to_str().is_ok(),
            true
        );

        if let Some(s) = arg_type.as_struct() {
            assert_eq!(unsafe { CStr::from_ptr(s.name) }.to_str().is_ok(), true);

            let field_names =
                unsafe { slice::from_raw_parts(s.field_names, s.num_fields as usize) };

            for field_name in field_names {
                assert_eq!(
                    unsafe { CStr::from_ptr(*field_name) }.to_str().is_ok(),
                    true
                );
            }
        }
    }
    assert_eq!(
        unsafe { CStr::from_ptr((*foo_func.signature.return_type).name) }
            .to_str()
            .is_ok(),
        true
    );
}

#[test]
fn fields() {
    let mut driver = TestDriver::new(
        r#"
        struct(gc) Foo { a:int, b:int };
        fn main(foo:int):bool {
            let a = Foo { a: foo, b: foo };
            a.a += a.b;
            let result = a;
            result.a += a.b;
            result.a == a.a
        }
    "#,
    );
    assert_invoke_eq!(bool, true, driver, "main", 48);
}

#[test]
fn field_crash() {
    let mut driver = TestDriver::new(
        r#"
    struct(gc) Foo { a: int };

    fn main(c:int):int {
        let b = Foo { a: c + 5 }
        b.a
    }
    "#,
    );
    assert_invoke_eq!(i64, 15, driver, "main", 10);
}

#[test]
fn marshal_struct() {
    let mut driver = TestDriver::new(
        r#"
    struct(gc) Foo { a: int, b: bool, c: float, };
    struct Bar(Foo);

    fn foo_new(a: int, b: bool, c: float): Foo {
        Foo { a, b, c, }
    }
    fn bar_new(foo: Foo): Bar {
        Bar(foo)
    }

    fn foo_a(foo: Foo):int { foo.a }
    fn foo_b(foo: Foo):bool { foo.b }
    fn foo_c(foo: Foo):float { foo.c }
    "#,
    );

    let a = 3i64;
    let b = true;
    let c = 1.23f64;
    let mut foo: Struct = invoke_fn!(driver.runtime, "foo_new", a, b, c).unwrap();
    assert_eq!(Ok(a), foo.get::<i64>("a"));
    assert_eq!(Ok(b), foo.get::<bool>("b"));
    assert_eq!(Ok(c), foo.get::<f64>("c"));

    let d = 6i64;
    let e = false;
    let f = 4.56f64;
    foo.set("a", d).unwrap();
    foo.set("b", e).unwrap();
    foo.set("c", f).unwrap();

    assert_eq!(Ok(d), foo.get::<i64>("a"));
    assert_eq!(Ok(e), foo.get::<bool>("b"));
    assert_eq!(Ok(f), foo.get::<f64>("c"));

    assert_eq!(Ok(d), foo.replace("a", a));
    assert_eq!(Ok(e), foo.replace("b", b));
    assert_eq!(Ok(f), foo.replace("c", c));

    assert_eq!(Ok(a), foo.get::<i64>("a"));
    assert_eq!(Ok(b), foo.get::<bool>("b"));
    assert_eq!(Ok(c), foo.get::<f64>("c"));

    assert_invoke_eq!(i64, a, driver, "foo_a", foo.clone());
    assert_invoke_eq!(bool, b, driver, "foo_b", foo.clone());
    assert_invoke_eq!(f64, c, driver, "foo_c", foo.clone());

    let mut bar: Struct = invoke_fn!(driver.runtime, "bar_new", foo.clone()).unwrap();
    let foo2 = bar.get::<Struct>("0").unwrap();
    assert_eq!(Ok(a), foo2.get::<i64>("a"));
    assert_eq!(foo2.get::<bool>("b"), foo.get::<bool>("b"));
    assert_eq!(foo2.get::<f64>("c"), foo.get::<f64>("c"));

    // Specify invalid return type
    let bar_err = bar.get::<i64>("0");
    assert!(bar_err.is_err());

    // Specify invalid argument type
    let bar_err = bar.replace("0", 1i64);
    assert!(bar_err.is_err());

    // Specify invalid argument type
    let bar_err = bar.set("0", 1i64);
    assert!(bar_err.is_err());

    // Specify invalid return type
    let bar_err: Result<i64, _> = invoke_fn!(driver.runtime, "bar_new", foo);
    assert!(bar_err.is_err());

    // Pass invalid struct type
    let bar_err: Result<Struct, _> = invoke_fn!(driver.runtime, "bar_new", bar);
    assert!(bar_err.is_err());
}

#[test]
fn hotreload_struct_decl() {
    let mut driver = TestDriver::new(
        r#"
    struct(value) Args {
        n: int,
        foo: Bar,
    }
    
    struct(gc) Bar {
        m: float,
    }

    fn args(): Args {
        Args { n: 3, foo: Bar { m: 1.0 }, }
    }
    "#,
    );
    driver.update(
        r#"
    struct(value) Args {
        n: int,
        foo: Bar,
    }
    
    struct(gc) Bar {
        m: int,
    }

    fn args(): Args {
        Args { n: 3, foo: Bar { m: 1 }, }
    }
    "#,
    );
}

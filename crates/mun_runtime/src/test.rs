use crate::function::IntoFunctionInfo;
use crate::{
    ArgumentReflection, RetryResultExt, ReturnTypeReflection, Runtime, RuntimeBuilder, StructRef,
};
use mun_compiler::{ColorChoice, Config, Driver, FileId, PathOrInline, RelativePathBuf};
use std::cell::RefCell;
use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;
use std::thread::sleep;
use std::time::Duration;

/// Implements a compiler and runtime in one that can invoke functions. Use of the TestDriver
/// enables quick testing of Mun constructs in the runtime with hot-reloading support.
struct TestDriver {
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
    pub fn spawn(&mut self) -> RuntimeOrBuilder {
        let previous = std::mem::replace(self, RuntimeOrBuilder::Pending);
        let runtime = match previous {
            RuntimeOrBuilder::Runtime(runtime) => runtime,
            RuntimeOrBuilder::Builder(builder) => Rc::new(RefCell::new(builder.spawn().unwrap())),
            _ => unreachable!(),
        };
        std::mem::replace(self, RuntimeOrBuilder::Runtime(runtime))
    }
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
        let (mut driver, file_id) = Driver::with_file(config, input).unwrap();
        let mut err_stream = mun_compiler::StandardStream::stderr(ColorChoice::Auto);
        if driver.emit_diagnostics(&mut err_stream).unwrap() {
            err_stream.flush().unwrap();
            panic!("compiler errors..")
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

    /// Updates the text of the Mun source and ensures that the generated assembly has been reloaded.
    fn update(&mut self, text: &str) {
        self.runtime_mut(); // Ensures that the runtime is spawned prior to the update
        self.driver.set_file_text(self.file_id, text);
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
    pub fn insert_fn<S: AsRef<str>, F: IntoFunctionInfo>(mut self, name: S, func: F) -> Self {
        match &mut self.runtime {
            RuntimeOrBuilder::Builder(builder) => builder.insert_fn(name, func),
            _ => unreachable!(),
        };
        self
    }

    /// Returns the `Runtime` used by this instance
    fn runtime_mut(&mut self) -> &mut Rc<RefCell<Runtime>> {
        self.runtime.spawn();
        match &mut self.runtime {
            RuntimeOrBuilder::Runtime(r) => r,
            _ => unreachable!(),
        }
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
        pub fn main() {}
    ",
    );
    assert_invoke_eq!((), (), driver, "main");
}

#[test]
fn return_value() {
    let mut driver = TestDriver::new(
        r"
        pub fn main():int { 3 }
    ",
    );
    assert_invoke_eq!(i64, 3, driver, "main");
}

#[test]
fn arguments() {
    let mut driver = TestDriver::new(
        r"
        pub fn main(a:int, b:int):int { a+b }
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
        pub fn add(a:int, b:int):int { a+b }
        pub fn main(a:int, b:int):int { add(a,b) }
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
        pub fn equal(a:int, b:int):bool                 { a==b }
        pub fn equalf(a:float, b:float):bool            { a==b }
        pub fn not_equal(a:int, b:int):bool             { a!=b }
        pub fn not_equalf(a:float, b:float):bool        { a!=b }
        pub fn less(a:int, b:int):bool                  { a<b }
        pub fn lessf(a:float, b:float):bool             { a<b }
        pub fn greater(a:int, b:int):bool               { a>b }
        pub fn greaterf(a:float, b:float):bool          { a>b }
        pub fn less_equal(a:int, b:int):bool            { a<=b }
        pub fn less_equalf(a:float, b:float):bool       { a<=b }
        pub fn greater_equal(a:int, b:int):bool         { a>=b }
        pub fn greater_equalf(a:float, b:float):bool    { a>=b }
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
    pub fn fibonacci(n:int):int {
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
    pub fn fibonacci(n:int):int {
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
    pub fn fibonacci(n:int):int {
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
    pub fn fibonacci(n:int):int {
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
    pub fn test_true():bool {
        true
    }

    pub fn test_false():bool {
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
    pub fn main():int { 5 }
    ",
    );
    assert_invoke_eq!(i64, 5, driver, "main");
    driver.update(
        r"
    pub fn main():int { 10 }
    ",
    );
    assert_invoke_eq!(i64, 10, driver, "main");
}

#[test]
fn compiler_valid_utf8() {
    use std::ffi::CStr;
    use std::slice;

    let mut driver = TestDriver::new(
        r#"
    struct Foo {
        a: int,
    }

    pub fn foo(n:Foo):bool { false }
    "#,
    );

    let borrowed = driver.runtime_mut().borrow();
    let foo_func = borrowed.get_function_info("foo").unwrap();
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
        pub fn main(foo:int):bool {
            let a = Foo { a: foo, b: foo };
            a.a += a.b;
            let result = a;
            result.a += a.b;
            result.a == a.a
        }
    "#,
    );
    assert_invoke_eq!(bool, true, driver, "main", 48isize);
}

#[test]
fn field_crash() {
    let mut driver = TestDriver::new(
        r#"
    struct(gc) Foo { a: int };

    pub fn main(c:int):int {
        let b = Foo { a: c + 5 }
        b.a
    }
    "#,
    );
    assert_invoke_eq!(i64, 15, driver, "main", 10isize);
}

#[test]
fn marshal_struct() {
    let mut driver = TestDriver::new(
        r#"
    struct(value) Foo { a: int, b: bool };
    struct Bar(int, bool);
    struct(value) Baz(Foo);
    struct(gc) Qux(Bar);

    pub fn foo_new(a: int, b: bool): Foo {
        Foo { a, b, }
    }
    pub fn bar_new(a: int, b: bool): Bar {
        Bar(a, b)
    }
    pub fn baz_new(foo: Foo): Baz {
        Baz(foo)
    }
    pub fn qux_new(bar: Bar): Qux {
        Qux(bar)
    }
    pub fn baz_new_transitive(foo_a: int, foo_b: bool) : Baz {
        Baz(foo_new(foo_a, foo_b))
    }
    "#,
    );

    struct TestData<T>(T, T);

    fn test_field<
        T: Copy + std::fmt::Debug + PartialEq + ArgumentReflection + ReturnTypeReflection,
    >(
        s: &mut StructRef,
        data: &TestData<T>,
        field_name: &str,
    ) {
        assert_eq!(Ok(data.0), s.get::<T>(field_name));
        s.set(field_name, data.1).unwrap();
        assert_eq!(Ok(data.1), s.replace(field_name, data.0));
        assert_eq!(Ok(data.0), s.get::<T>(field_name));
    }

    let int_data = TestData(3i64, 6i64);
    let bool_data = TestData(true, false);

    // Verify that struct marshalling works for fundamental types
    let mut foo: StructRef =
        invoke_fn!(driver.runtime_mut(), "foo_new", int_data.0, bool_data.0).unwrap();
    test_field(&mut foo, &int_data, "a");
    test_field(&mut foo, &bool_data, "b");

    let mut bar: StructRef =
        invoke_fn!(driver.runtime_mut(), "bar_new", int_data.0, bool_data.0).unwrap();
    test_field(&mut bar, &int_data, "0");
    test_field(&mut bar, &bool_data, "1");

    fn test_struct(runtime: &Runtime, s: &mut StructRef, c1: StructRef, c2: StructRef) {
        let field_names: Vec<String> = StructRef::type_info(&c1, runtime)
            .as_struct()
            .unwrap()
            .field_names()
            .map(|n| n.to_string())
            .collect();

        let int_value = c2.get::<i64>(&field_names[0]);
        let bool_value = c2.get::<bool>(&field_names[1]);
        s.set("0", c2).unwrap();

        let c2 = s.get::<StructRef>("0").unwrap();
        assert_eq!(c2.get::<i64>(&field_names[0]), int_value);
        assert_eq!(c2.get::<bool>(&field_names[1]), bool_value);

        let int_value = c1.get::<i64>(&field_names[0]);
        let bool_value = c1.get::<bool>(&field_names[1]);
        s.replace("0", c1).unwrap();

        let c1 = s.get::<StructRef>("0").unwrap();
        assert_eq!(c1.get::<i64>(&field_names[0]), int_value);
        assert_eq!(c1.get::<bool>(&field_names[1]), bool_value);
    }

    // Verify that struct marshalling works for struct types
    let mut baz: StructRef = invoke_fn!(driver.runtime_mut(), "baz_new", foo).unwrap();
    let c1: StructRef =
        invoke_fn!(driver.runtime_mut(), "foo_new", int_data.0, bool_data.0).unwrap();
    let c2: StructRef =
        invoke_fn!(driver.runtime_mut(), "foo_new", int_data.1, bool_data.1).unwrap();
    test_struct(&driver.runtime_mut().borrow(), &mut baz, c1, c2);

    let mut qux: StructRef = invoke_fn!(driver.runtime_mut(), "qux_new", bar).unwrap();
    let c1: StructRef =
        invoke_fn!(driver.runtime_mut(), "bar_new", int_data.0, bool_data.0).unwrap();
    let c2: StructRef =
        invoke_fn!(driver.runtime_mut(), "bar_new", int_data.1, bool_data.1).unwrap();
    test_struct(&driver.runtime_mut().borrow(), &mut qux, c1, c2);

    // Verify the dispatch table works when a marshallable wrapper function exists alongside the
    // original function.
    let mut baz2: StructRef = invoke_fn!(
        driver.runtime_mut(),
        "baz_new_transitive",
        int_data.0,
        bool_data.0
    )
    .wait();
    let c1: StructRef =
        invoke_fn!(driver.runtime_mut(), "foo_new", int_data.0, bool_data.0).unwrap();
    let c2: StructRef =
        invoke_fn!(driver.runtime_mut(), "foo_new", int_data.1, bool_data.1).unwrap();
    test_struct(&driver.runtime_mut().borrow(), &mut baz2, c1, c2);

    fn test_shallow_copy<
        T: Copy + std::fmt::Debug + PartialEq + ArgumentReflection + ReturnTypeReflection,
    >(
        s1: &mut StructRef,
        s2: &StructRef,
        data: &TestData<T>,
        field_name: &str,
    ) {
        assert_eq!(s1.get::<T>(field_name), s2.get::<T>(field_name));
        s1.set(field_name, data.1).unwrap();
        assert_ne!(s1.get::<T>(field_name), s2.get::<T>(field_name));
        s1.replace(field_name, data.0).unwrap();
        assert_eq!(s1.get::<T>(field_name), s2.get::<T>(field_name));
    }

    // Verify that StructRef::get makes a shallow copy of a struct
    let mut foo = baz.get::<StructRef>("0").unwrap();
    let foo2 = baz.get::<StructRef>("0").unwrap();
    test_shallow_copy(&mut foo, &foo2, &int_data, "a");
    test_shallow_copy(&mut foo, &foo2, &bool_data, "b");

    let mut bar = qux.get::<StructRef>("0").unwrap();

    // Specify invalid return type
    let bar_err = bar.get::<f64>("0");
    assert!(bar_err.is_err());

    // Specify invalid argument type
    let bar_err = bar.replace("0", 1f64);
    assert!(bar_err.is_err());

    // Specify invalid argument type
    let bar_err = bar.set("0", 1f64);
    assert!(bar_err.is_err());

    // Specify invalid return type
    let bar_err: Result<i64, _> = invoke_fn!(driver.runtime_mut(), "baz_new", foo);
    assert!(bar_err.is_err());

    // Pass invalid struct type
    let bar_err: Result<StructRef, _> = invoke_fn!(driver.runtime_mut(), "baz_new", bar);
    assert!(bar_err.is_err());
}

#[test]
fn hotreload_struct_decl() {
    let mut driver = TestDriver::new(
        r#"
    struct(gc) Args {
        n: int,
        foo: Bar,
    }
    
    struct(gc) Bar {
        m: float,
    }

    pub fn args(): Args {
        Args { n: 3, foo: Bar { m: 1.0 }, }
    }
    "#,
    );
    driver.update(
        r#"
    struct(gc) Args {
        n: int,
        foo: Bar,
    }
    
    struct(gc) Bar {
        m: int,
    }

    pub fn args(): Args {
        Args { n: 3, foo: Bar { m: 1 }, }
    }
    "#,
    );
}

#[test]
fn extern_fn() {
    extern "C" fn add_int(a: isize, b: isize) -> isize {
        dbg!("add_int is called!");
        a + b + 9
    }

    let mut driver = TestDriver::new(
        r#"
    extern fn add(a: int, b: int): int;
    pub fn main(): int {
        add(3,4)
    }
    "#,
    )
    .insert_fn("add", add_int as extern "C" fn(isize, isize) -> isize);
    assert_invoke_eq!(isize, 16, driver, "main");
}

#[test]
#[should_panic]
fn extern_fn_missing() {
    let mut driver = TestDriver::new(
        r#"
    extern fn add(a: int, b: int): int;
    pub fn main(): int { add(3,4) }
    "#,
    );
    assert_invoke_eq!(isize, 16, driver, "main");
}

#[test]
#[should_panic]
fn extern_fn_invalid_sig() {
    extern "C" fn add_int(_a: i8, _b: isize) -> isize {
        3
    }

    let mut driver = TestDriver::new(
        r#"
    extern fn add(a: int, b: int): int;
    pub fn main(): int { add(3,4) }
    "#,
    )
    .insert_fn("add", add_int as extern "C" fn(i8, isize) -> isize);
    assert_invoke_eq!(isize, 16, driver, "main");
}

#[test]
fn test_primitive_types() {
    let mut driver = TestDriver::new(
        r#"
    struct Primitives {
        a:u8,
        b:u16,
        c:u32,
        d:u64,

        e:i8,
        f:i16,
        g:i32,
        h:i64,

        i:f32,
        j:f64,

        k: int,
        l: uint,
        m: float
    }

    pub fn new_primitives(a:u8, b:u16, c:u32, d:u64, e:i8, f:i16, g:i32, h:i64, i:f32, j:f64, k: int, l: uint, m: float): Primitives {
        Primitives { a:a, b:b, c:c, d:d, e:e, f:f, g:g, h:h, i:i, j:j, k:k, l:l, m:m }
    }
    "#,
    );

    fn test_field<
        T: Copy + std::fmt::Debug + PartialEq + ArgumentReflection + ReturnTypeReflection,
    >(
        s: &mut StructRef,
        data: (T, T),
        field_name: &str,
    ) {
        assert_eq!(Ok(data.0), s.get::<T>(field_name));
        s.set(field_name, data.1).unwrap();
        assert_eq!(Ok(data.1), s.replace(field_name, data.0));
        assert_eq!(Ok(data.0), s.get::<T>(field_name));
    }

    let mut foo: StructRef = invoke_fn!(
        driver.runtime_mut(),
        "new_primitives",
        1u8,
        2u16,
        3u32,
        4u64,
        5i8,
        6i16,
        7i32,
        8i64,
        9.0f32,
        10.0f64,
        11isize,
        12usize,
        13.0f64
    )
    .unwrap();

    test_field(&mut foo, (1u8, 100u8), "a");
    test_field(&mut foo, (2u16, 101u16), "b");
    test_field(&mut foo, (3u32, 102u32), "c");
    test_field(&mut foo, (4u64, 103u64), "d");
    test_field(&mut foo, (5i8, 104i8), "e");
    test_field(&mut foo, (6i16, 105i16), "f");
    test_field(&mut foo, (7i32, 106i32), "g");
    test_field(&mut foo, (8i64, 107i64), "h");
    test_field(&mut foo, (9f32, 108f32), "i");
    test_field(&mut foo, (10f64, 109f64), "j");
    test_field(&mut foo, (11isize, 110isize), "k");
    test_field(&mut foo, (12usize, 111usize), "l");
    test_field(&mut foo, (13f64, 112f64), "m");
}

#[test]
fn gc_trace() {
    let mut driver = TestDriver::new(
        r#"
    pub struct Foo {
        quz: float,
        bar: Bar,
    }

    pub struct Bar {
        baz: int
    }

    pub fn new_foo(): Foo {
        Foo {
            quz: 1.0,
            bar: Bar {
                baz: 3
            }
        }
    }
    "#,
    );

    let value: StructRef = invoke_fn!(driver.runtime_mut(), "new_foo").unwrap();

    assert_eq!(driver.runtime_mut().borrow().gc_collect(), false);
    assert!(driver.runtime_mut().borrow().gc_stats().allocated_memory > 0);

    drop(value);

    assert_eq!(driver.runtime_mut().borrow().gc_collect(), true);
    assert_eq!(driver.runtime_mut().borrow().gc_stats().allocated_memory, 0);
}

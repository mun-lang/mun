use mun_runtime::{ArgumentReflection, Marshal, ReturnTypeReflection, StructRef};

use mun_test::CompileAndRunTestDriver;

#[macro_use]
mod util;

#[test]
fn compile_and_run() {
    let driver = CompileAndRunTestDriver::new(
        r"
        pub fn main() {}
    ",
        |builder| builder,
    )
    .expect("Failed to build test driver");

    assert_invoke_eq!((), (), driver, "main");
}

#[test]
fn return_value() {
    let driver = CompileAndRunTestDriver::new(
        r"
        pub fn main()->i32 { 3 }
    ",
        |builder| builder,
    )
    .expect("Failed to build test driver");

    assert_invoke_eq!(i32, 3, driver, "main");
}

#[test]
fn arguments() {
    let driver = CompileAndRunTestDriver::new(
        r"
        pub fn main(a:i32, b:i32)->i32 { a+b }
    ",
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a: i32 = 52;
    let b: i32 = 746;
    assert_invoke_eq!(i32, a + b, driver, "main", a, b);
}

#[test]
fn dispatch_table() {
    let driver = CompileAndRunTestDriver::new(
        r"
        pub fn add(a:i32, b:i32)->i32 { a+b }
        pub fn main(a:i32, b:i32)->i32 { add(a,b) }
    ",
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a: i32 = 52;
    let b: i32 = 746;
    assert_invoke_eq!(i32, a + b, driver, "main", a, b);

    let a: i32 = 6274;
    let b: i32 = 72;
    assert_invoke_eq!(i32, a + b, driver, "add", a, b);
}

#[test]
fn booleans() {
    let driver = CompileAndRunTestDriver::new(
        r#"
        pub fn equal(a:i64, b:i64)->bool                 { a==b }
        pub fn equalf(a:f64, b:f64)->bool            { a==b }
        pub fn not_equal(a:i64, b:i64)->bool             { a!=b }
        pub fn not_equalf(a:f64, b:f64)->bool        { a!=b }
        pub fn less(a:i64, b:i64)->bool                  { a<b }
        pub fn lessf(a:f64, b:f64)->bool             { a<b }
        pub fn greater(a:i64, b:i64)->bool               { a>b }
        pub fn greaterf(a:f64, b:f64)->bool          { a>b }
        pub fn less_equal(a:i64, b:i64)->bool            { a<=b }
        pub fn less_equalf(a:f64, b:f64)->bool       { a<=b }
        pub fn greater_equal(a:i64, b:i64)->bool         { a>=b }
        pub fn greater_equalf(a:f64, b:f64)->bool    { a>=b }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

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
    let driver = CompileAndRunTestDriver::new(
        r#"
    pub fn fibonacci(n:i64)->i64 {
        if n <= 1 {
            n
        } else {
            fibonacci(n-1) + fibonacci(n-2)
        }
    }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    assert_invoke_eq!(i64, 5, driver, "fibonacci", 5i64);
    assert_invoke_eq!(i64, 89, driver, "fibonacci", 11i64);
    assert_invoke_eq!(i64, 987, driver, "fibonacci", 16i64);
}

#[test]
fn fibonacci_loop() {
    let driver = CompileAndRunTestDriver::new(
        r#"
    pub fn fibonacci(n:i64)->i64 {
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
        |builder| builder,
    )
    .expect("Failed to build test driver");

    assert_invoke_eq!(i64, 5, driver, "fibonacci", 5i64);
    assert_invoke_eq!(i64, 89, driver, "fibonacci", 11i64);
    assert_invoke_eq!(i64, 987, driver, "fibonacci", 16i64);
    assert_invoke_eq!(i64, 46368, driver, "fibonacci", 24i64);
}

#[test]
fn fibonacci_loop_break() {
    let driver = CompileAndRunTestDriver::new(
        r#"
    pub fn fibonacci(n:i64)->i64 {
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
        |builder| builder,
    )
    .expect("Failed to build test driver");

    assert_invoke_eq!(i64, 5, driver, "fibonacci", 5i64);
    assert_invoke_eq!(i64, 89, driver, "fibonacci", 11i64);
    assert_invoke_eq!(i64, 987, driver, "fibonacci", 16i64);
    assert_invoke_eq!(i64, 46368, driver, "fibonacci", 24i64);
}

#[test]
fn fibonacci_while() {
    let driver = CompileAndRunTestDriver::new(
        r#"
    pub fn fibonacci(n:i64)->i64 {
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
        |builder| builder,
    )
    .expect("Failed to build test driver");

    assert_invoke_eq!(i64, 5, driver, "fibonacci", 5i64);
    assert_invoke_eq!(i64, 89, driver, "fibonacci", 11i64);
    assert_invoke_eq!(i64, 987, driver, "fibonacci", 16i64);
    assert_invoke_eq!(i64, 46368, driver, "fibonacci", 24i64);
}

#[test]
fn true_is_true() {
    let driver = CompileAndRunTestDriver::new(
        r#"
    pub fn test_true()->bool {
        true
    }

    pub fn test_false()->bool {
        false
    }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    assert_invoke_eq!(bool, true, driver, "test_true");
    assert_invoke_eq!(bool, false, driver, "test_false");
}

#[test]
fn compiler_valid_utf8() {
    use std::ffi::CStr;
    use std::slice;

    let driver = CompileAndRunTestDriver::new(
        r#"
    pub struct Foo {
        a: i32,
    }

    pub fn foo(n:Foo)->bool { false }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let runtime = driver.runtime();
    let runtime_ref = runtime.borrow();
    let foo_func = runtime_ref.get_function_definition("foo").unwrap();
    assert_eq!(
        unsafe { CStr::from_ptr(foo_func.prototype.name) }
            .to_str()
            .is_ok(),
        true
    );

    for arg_type in foo_func.prototype.signature.arg_types() {
        assert_eq!(
            unsafe { CStr::from_ptr(arg_type.name) }.to_str().is_ok(),
            true
        );

        if let Some(s) = arg_type.as_struct() {
            let field_names = unsafe { slice::from_raw_parts(s.field_names, s.num_fields()) };

            for field_name in field_names {
                assert_eq!(
                    unsafe { CStr::from_ptr(*field_name) }.to_str().is_ok(),
                    true
                );
            }
        }
    }
    assert_eq!(
        unsafe {
            CStr::from_ptr(
                foo_func
                    .prototype
                    .signature
                    .return_type()
                    .expect("Missing return type")
                    .name,
            )
        }
        .to_str()
        .is_ok(),
        true
    );
}

#[test]
fn fields() {
    let driver = CompileAndRunTestDriver::new(
        r#"
        struct(gc) Foo { a:i32, b:i32 };
        pub fn main(foo:i32)->bool {
            let a = Foo { a: foo, b: foo };
            a.a += a.b;
            let result = a;
            result.a += a.b;
            result.a == a.a
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    assert_invoke_eq!(bool, true, driver, "main", 48i32);
}

#[test]
fn field_crash() {
    let driver = CompileAndRunTestDriver::new(
        r#"
    struct(gc) Foo { a: i32 };

    pub fn main(c:i32)->i32 {
        let b = Foo { a: c + 5 }
        b.a
    }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    assert_invoke_eq!(i32, 15, driver, "main", 10i32);
}

#[test]
fn marshal_struct() {
    let driver = CompileAndRunTestDriver::new(
        r#"
    pub struct(value) Foo { a: i32, b: bool };
    pub struct Bar(i32, bool);
    pub struct(value) Baz(Foo);
    pub struct(gc) Qux(Bar);

    pub fn foo_new(a: i32, b: bool) -> Foo {
        Foo { a, b, }
    }
    pub fn bar_new(a: i32, b: bool) -> Bar {
        Bar(a, b)
    }
    pub fn baz_new(foo: Foo) -> Baz {
        Baz(foo)
    }
    pub fn qux_new(bar: Bar) -> Qux {
        Qux(bar)
    }
    pub fn baz_new_transitive(foo_a: i32, foo_b: bool) -> Baz {
        Baz(foo_new(foo_a, foo_b))
    }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    struct TestData<T>(T, T);

    fn test_field<
        't,
        T: 't
            + Copy
            + std::fmt::Debug
            + PartialEq
            + ArgumentReflection
            + ReturnTypeReflection
            + Marshal<'t>,
    >(
        s: &mut StructRef<'t>,
        data: &TestData<T>,
        field_name: &str,
    ) {
        assert_eq!(Ok(data.0), s.get::<T>(field_name));
        s.set(field_name, data.1).unwrap();
        assert_eq!(Ok(data.1), s.replace(field_name, data.0));
        assert_eq!(Ok(data.0), s.get::<T>(field_name));
    }

    let runtime = driver.runtime();
    let runtime_ref = runtime.borrow();

    let int_data = TestData(3i32, 6i32);
    let bool_data = TestData(true, false);

    // Verify that struct marshalling works for fundamental types
    let mut foo: StructRef = runtime_ref
        .invoke("foo_new", (int_data.0, bool_data.0))
        .unwrap();
    test_field(&mut foo, &int_data, "a");
    test_field(&mut foo, &bool_data, "b");

    let mut bar: StructRef = runtime_ref
        .invoke("bar_new", (int_data.0, bool_data.0))
        .unwrap();
    test_field(&mut bar, &int_data, "0");
    test_field(&mut bar, &bool_data, "1");

    fn test_struct<'t>(s: &mut StructRef<'t>, c1: StructRef<'t>, c2: StructRef<'t>) {
        let field_names: Vec<String> = c1
            .type_info()
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
    let mut baz: StructRef = runtime_ref.invoke("baz_new", (foo,)).unwrap();
    let c1: StructRef = runtime_ref
        .invoke("foo_new", (int_data.0, bool_data.0))
        .unwrap();
    let c2: StructRef = runtime_ref
        .invoke("foo_new", (int_data.1, bool_data.1))
        .unwrap();
    test_struct(&mut baz, c1, c2);

    let mut qux: StructRef = runtime_ref.invoke("qux_new", (bar,)).unwrap();
    let c1: StructRef = runtime_ref
        .invoke("bar_new", (int_data.0, bool_data.0))
        .unwrap();
    let c2: StructRef = runtime_ref
        .invoke("bar_new", (int_data.1, bool_data.1))
        .unwrap();
    test_struct(&mut qux, c1, c2);

    // Verify the dispatch table works when a marshallable wrapper function exists alongside the
    // original function.
    let mut baz2: StructRef = runtime_ref
        .invoke("baz_new_transitive", (int_data.0, bool_data.0))
        .unwrap();
    // TODO: Find an ergonomic solution for this:
    // .unwrap_or_else(|e| e.wait(&mut runtime_ref));

    let runtime_ref = runtime.borrow();
    let c1: StructRef = runtime_ref
        .invoke("foo_new", (int_data.0, bool_data.0))
        .unwrap();
    let c2: StructRef = runtime_ref
        .invoke("foo_new", (int_data.1, bool_data.1))
        .unwrap();
    test_struct(&mut baz2, c1, c2);

    fn test_shallow_copy<
        't,
        T: 't
            + Copy
            + std::fmt::Debug
            + PartialEq
            + ArgumentReflection
            + ReturnTypeReflection
            + Marshal<'t>,
    >(
        s1: &mut StructRef<'t>,
        s2: &StructRef<'t>,
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

    fn test_clone<
        't,
        T: 't
            + Copy
            + std::fmt::Debug
            + PartialEq
            + ArgumentReflection
            + ReturnTypeReflection
            + Marshal<'t>,
    >(
        s1: &mut StructRef<'t>,
        s2: &StructRef<'t>,
        data: &TestData<T>,
        field_name: &str,
    ) {
        assert_eq!(s1.get::<T>(field_name), s2.get::<T>(field_name));
        s1.set(field_name, data.1).unwrap();
        assert_eq!(s1.get::<T>(field_name), s2.get::<T>(field_name));
        s1.replace(field_name, data.0).unwrap();
        assert_eq!(s1.get::<T>(field_name), s2.get::<T>(field_name));
    }

    // Verify that StructRef::clone returns a `StructRef` to the same memory
    let mut foo = baz.get::<StructRef>("0").unwrap();
    let foo2 = foo.clone();
    test_clone(&mut foo, &foo2, &int_data, "a");
    test_clone(&mut foo, &foo2, &bool_data, "b");

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
    let bar_err: Result<i64, _> = runtime_ref.invoke("baz_new", (foo,));
    assert!(bar_err.is_err());

    // Pass invalid struct type
    let bar_err: Result<StructRef, _> = runtime_ref.invoke("baz_new", (bar,));
    assert!(bar_err.is_err());
}

#[test]
fn extern_fn() {
    extern "C" fn add_int(a: i32, b: i32) -> i32 {
        dbg!("add_int is called!");
        a + b + 9
    }

    let driver = CompileAndRunTestDriver::new(
        r#"
    extern fn add(a: i32, b: i32) -> i32;
    pub fn main() -> i32 {
        add(3,4)
    }
    "#,
        |builder| builder.insert_fn("add", add_int as extern "C" fn(i32, i32) -> i32),
    )
    .expect("Failed to build test driver");

    assert_invoke_eq!(i32, 16, driver, "main");
}

#[test]
#[should_panic]
fn extern_fn_missing() {
    let driver = CompileAndRunTestDriver::new(
        r#"
    extern fn add(a: i32, b: i32) -> i32;
    pub fn main() -> i32 { add(3,4) }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    assert_invoke_eq!(isize, 16, driver, "main");
}

#[test]
fn extern_fn_invalid_signature() {
    extern "C" fn add_int() -> i32 {
        0
    }

    let result = CompileAndRunTestDriver::new(
        r#"
    extern fn add(a: i32, b: i32) -> i32;
    pub fn main() -> i32 { add(3,4) }
    "#,
        |builder| builder.insert_fn("add", add_int as extern "C" fn() -> i32),
    );

    assert!(result.is_err());
}

#[test]
#[should_panic]
fn extern_fn_invalid_sig() {
    extern "C" fn add_int(_a: i8, _b: isize) -> isize {
        3
    }

    let driver = CompileAndRunTestDriver::new(
        r#"
    extern fn add(a: i32, b: i32) -> i32;
    pub fn main() -> i32 { add(3,4) }
    "#,
        |builder| builder.insert_fn("add", add_int as extern "C" fn(i8, isize) -> isize),
    )
    .expect("Failed to build test driver");

    assert_invoke_eq!(isize, 16, driver, "main");
}

#[test]
fn test_primitive_types() {
    let driver = CompileAndRunTestDriver::new(
        r#"
    pub struct Primitives {
        a:u8,
        b:u16,
        c:u32,
        d:u64,
        e:u128,

        f:i8,
        g:i16,
        h:i32,
        i:i64,
        j:i128,

        k:f32,
        l:f64,
    }

    pub fn new_primitives(a:u8, b:u16, c:u32, d:u64, e:u128, f:i8, g:i16, h:i32, i:i64, j:i128, k:f32, l:f64) -> Primitives {
        Primitives { a:a, b:b, c:c, d:d, e:e, f:f, g:g, h:h, i:i, j:j, k:k, l:l }
    }
    "#,
    |builder| builder
    )
    .expect("Failed to build test driver");

    fn test_field<
        't,
        T: 't
            + Copy
            + std::fmt::Debug
            + PartialEq
            + ArgumentReflection
            + ReturnTypeReflection
            + Marshal<'t>,
    >(
        s: &mut StructRef<'t>,
        data: (T, T),
        field_name: &str,
    ) {
        assert_eq!(Ok(data.0), s.get::<T>(field_name));
        s.set(field_name, data.1).unwrap();
        assert_eq!(Ok(data.1), s.replace(field_name, data.0));
        assert_eq!(Ok(data.0), s.get::<T>(field_name));
    }

    let runtime = driver.runtime();
    let runtime_ref = runtime.borrow();

    let mut foo: StructRef = runtime_ref
        .invoke(
            "new_primitives",
            (
                1u8, 2u16, 3u32, 4u64, 5u128, 6i8, 7i16, 8i32, 9i64, 10i128, 11.0f32, 12.0f64,
            ),
        )
        .unwrap();

    test_field(&mut foo, (1u8, 100u8), "a");
    test_field(&mut foo, (2u16, 101u16), "b");
    test_field(&mut foo, (3u32, 102u32), "c");
    test_field(&mut foo, (4u64, 103u64), "d");
    test_field(&mut foo, (5u128, 104u128), "e");
    test_field(&mut foo, (6i8, 105i8), "f");
    test_field(&mut foo, (7i16, 106i16), "g");
    test_field(&mut foo, (8i32, 107i32), "h");
    test_field(&mut foo, (9i64, 108i64), "i");
    test_field(&mut foo, (10i128, 109i128), "j");
    test_field(&mut foo, (11f32, 110f32), "k");
    test_field(&mut foo, (12f64, 111f64), "l");
}

#[test]
fn can_add_external_without_return() {
    extern "C" fn foo(a: i32) {
        println!("{}", a);
    }

    let driver = CompileAndRunTestDriver::new(
        r#"
    extern fn foo(a: i32,);
    pub fn main(){ foo(3); }
    "#,
        |builder| builder.insert_fn("foo", foo as extern "C" fn(i32) -> ()),
    )
    .expect("Failed to build test driver");

    let runtime = driver.runtime();
    let runtime_ref = runtime.borrow();

    let _: () = runtime_ref.invoke("main", ()).unwrap();
}

#[test]
fn signed_and_unsigned_rem() {
    let driver = CompileAndRunTestDriver::new(
        r#"
    pub fn signed() -> i32 {
        (0 - 2) % 5
    }

    pub fn unsigned() -> i32 {
        2 % 5
    }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    assert_invoke_eq!(i32, -2, driver, "signed");
    assert_invoke_eq!(i32, 2, driver, "unsigned");
}

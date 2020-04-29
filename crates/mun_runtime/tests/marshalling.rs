use mun_runtime::{
    invoke_fn, ArgumentReflection, RetryResultExt, ReturnTypeReflection, Runtime, StructRef,
};

#[macro_use]
mod util;

use util::*;

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
        pub fn main()->int { 3 }
    ",
    );
    assert_invoke_eq!(i64, 3, driver, "main");
}

#[test]
fn arguments() {
    let mut driver = TestDriver::new(
        r"
        pub fn main(a:int, b:int)->int { a+b }
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
        pub fn add(a:int, b:int)->int { a+b }
        pub fn main(a:int, b:int)->int { add(a,b) }
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
        pub fn equal(a:int, b:int)->bool                 { a==b }
        pub fn equalf(a:float, b:float)->bool            { a==b }
        pub fn not_equal(a:int, b:int)->bool             { a!=b }
        pub fn not_equalf(a:float, b:float)->bool        { a!=b }
        pub fn less(a:int, b:int)->bool                  { a<b }
        pub fn lessf(a:float, b:float)->bool             { a<b }
        pub fn greater(a:int, b:int)->bool               { a>b }
        pub fn greaterf(a:float, b:float)->bool          { a>b }
        pub fn less_equal(a:int, b:int)->bool            { a<=b }
        pub fn less_equalf(a:float, b:float)->bool       { a<=b }
        pub fn greater_equal(a:int, b:int)->bool         { a>=b }
        pub fn greater_equalf(a:float, b:float)->bool    { a>=b }
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
    pub fn fibonacci(n:int)->int {
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
    pub fn fibonacci(n:int)->int {
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
    pub fn fibonacci(n:int)->int {
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
    pub fn fibonacci(n:int)->int {
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
    pub fn test_true()->bool {
        true
    }

    pub fn test_false()->bool {
        false
    }
    "#,
    );
    assert_invoke_eq!(bool, true, driver, "test_true");
    assert_invoke_eq!(bool, false, driver, "test_false");
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

    pub fn foo(n:Foo)->bool { false }
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
        pub fn main(foo:int)->bool {
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

    pub fn main(c:int)->int {
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

    pub fn foo_new(a: int, b: bool) -> Foo {
        Foo { a, b, }
    }
    pub fn bar_new(a: int, b: bool) -> Bar {
        Bar(a, b)
    }
    pub fn baz_new(foo: Foo) -> Baz {
        Baz(foo)
    }
    pub fn qux_new(bar: Bar) -> Qux {
        Qux(bar)
    }
    pub fn baz_new_transitive(foo_a: int, foo_b: bool) -> Baz {
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

    fn test_clone<
        T: Copy + std::fmt::Debug + PartialEq + ArgumentReflection + ReturnTypeReflection,
    >(
        s1: &mut StructRef,
        s2: &StructRef,
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
    let bar_err: Result<i64, _> = invoke_fn!(driver.runtime_mut(), "baz_new", foo);
    assert!(bar_err.is_err());

    // Pass invalid struct type
    let bar_err: Result<StructRef, _> = invoke_fn!(driver.runtime_mut(), "baz_new", bar);
    assert!(bar_err.is_err());
}

#[test]
fn extern_fn() {
    extern "C" fn add_int(a: isize, b: isize) -> isize {
        dbg!("add_int is called!");
        a + b + 9
    }

    let mut driver = TestDriver::new(
        r#"
    extern fn add(a: int, b: int) -> int;
    pub fn main() -> int {
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
    extern fn add(a: int, b: int) -> int;
    pub fn main() -> int { add(3,4) }
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
    extern fn add(a: int, b: int) -> int;
    pub fn main() -> int { add(3,4) }
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
        e:u128,

        f:i8,
        g:i16,
        h:i32,
        i:i64,
        j:i128,

        k:f32,
        l:f64,

        m: int,
        n: uint,
        o: float
    }

    pub fn new_primitives(a:u8, b:u16, c:u32, d:u64, e:u128, f:i8, g:i16, h:i32, i:i64, j:i128, k:f32, l:f64, m: int, n: uint, o: float) -> Primitives {
        Primitives { a:a, b:b, c:c, d:d, e:e, f:f, g:g, h:h, i:i, j:j, k:k, l:l, m:m, n:n, o:o }
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
        5u128,
        6i8,
        7i16,
        8i32,
        9i64,
        10i128,
        11.0f32,
        12.0f64,
        13isize,
        14usize,
        15.0f64
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
    test_field(&mut foo, (13isize, 112isize), "m");
    test_field(&mut foo, (14usize, 113usize), "n");
    test_field(&mut foo, (15f64, 114f64), "o");
}

#[test]
fn can_add_external_without_return() {
    extern "C" fn foo(a: i64) {
        println!("{}", a);
    }

    let mut driver = TestDriver::new(
        r#"
    extern fn foo(a: int,);
    pub fn main(){ foo(3); }
    "#,
    )
    .insert_fn("foo", foo as extern "C" fn(i64) -> ());
    let _: () = invoke_fn!(driver.runtime_mut(), "main").unwrap();
}

#[test]
fn signed_and_unsigned_rem() {
    let mut driver = TestDriver::new(
        r#"
    pub fn signed() -> int {
        (0 - 2) % 5
    }

    pub fn unsigned() -> int {
        2 % 5
    }
    "#,
    );

    assert_invoke_eq!(i64, -2, driver, "signed");
    assert_invoke_eq!(i64, 2, driver, "unsigned");
}

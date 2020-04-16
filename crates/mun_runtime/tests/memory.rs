use mun_runtime::{invoke_fn, StructRef};

#[macro_use]
mod util;

use util::*;

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

    pub fn new_foo() -> Foo {
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

#[test]
fn map_struct_insert_field1() {
    let mut driver = TestDriver::new(
        r#"
        struct Foo {
            b: int,
            c: float,
        }

        pub fn foo_new(b: int, c: float) -> Foo {
            Foo { b, c }
        }
    "#,
    );

    let b = 5i64;
    let c = 3.0f64;
    let foo: StructRef = invoke_fn!(driver.runtime_mut(), "foo_new", b, c).unwrap();

    driver.update(
        r#"
        struct Foo {
            a: int,
            b: int,
            c: float,
        }
    "#,
    );
    assert_eq!(foo.get::<i64>("a").unwrap(), 0);
    assert_eq!(foo.get::<i64>("b").unwrap(), b);
    assert_eq!(foo.get::<f64>("c").unwrap(), c);
}

#[test]
fn map_struct_insert_field2() {
    let mut driver = TestDriver::new(
        r#"
        struct Foo {
            a: int,
            c: float,
        }

        pub fn foo_new(a: int, c: float) -> Foo {
            Foo { a, c }
        }
    "#,
    );

    let a = 5i64;
    let c = 3.0f64;
    let foo: StructRef = invoke_fn!(driver.runtime_mut(), "foo_new", a, c).unwrap();

    driver.update(
        r#"
        struct Foo {
            a: int,
            b: float,
            c: float,
        }
    "#,
    );
    assert_eq!(foo.get::<i64>("a").unwrap(), a);
    assert_eq!(foo.get::<f64>("b").unwrap(), 0.0);
    assert_eq!(foo.get::<f64>("c").unwrap(), c);
}

#[test]
fn map_struct_insert_field3() {
    let mut driver = TestDriver::new(
        r#"
        struct Foo {
            a: int,
            b: float,
        }

        pub fn foo_new(a: int, b: float) -> Foo {
            Foo { a, b }
        }
    "#,
    );

    let a = 5i64;
    let b = 3.0f64;
    let foo: StructRef = invoke_fn!(driver.runtime_mut(), "foo_new", a, b).unwrap();

    driver.update(
        r#"
        struct Foo {
            a: int,
            b: float,
            c: float,
        }
    "#,
    );
    assert_eq!(foo.get::<i64>("a").unwrap(), a);
    assert_eq!(foo.get::<f64>("b").unwrap(), b);
    assert_eq!(foo.get::<f64>("c").unwrap(), 0.0);
}

#[test]
fn map_struct_remove_field1() {
    let mut driver = TestDriver::new(
        r#"
        struct Foo {
            a: float,
            b: float,
            c: int,
        }

        pub fn foo_new(a: float, b: float, c: int) -> Foo {
            Foo { a, b, c }
        }
    "#,
    );

    let a = 1.0f64;
    let b = 3.0f64;
    let c = 5i64;
    let foo: StructRef = invoke_fn!(driver.runtime_mut(), "foo_new", a, b, c).unwrap();

    driver.update(
        r#"
        struct Foo {
            c: int,
        }
    "#,
    );
    assert_eq!(foo.get::<i64>("c").unwrap(), c);
}

#[test]
fn map_struct_remove_field2() {
    let mut driver = TestDriver::new(
        r#"
        struct Foo {
            a: float,
            b: int,
            c: float,
        }

        pub fn foo_new(a: float, b: int, c: float) -> Foo {
            Foo { a, b, c }
        }
    "#,
    );

    let a = 1.0f64;
    let b = 5i64;
    let c = 3.0f64;
    let foo: StructRef = invoke_fn!(driver.runtime_mut(), "foo_new", a, b, c).unwrap();

    driver.update(
        r#"
        struct Foo {
            b: int,
        }
    "#,
    );
    assert_eq!(foo.get::<i64>("b").unwrap(), b);
}

#[test]
fn map_struct_remove_field3() {
    let mut driver = TestDriver::new(
        r#"
        struct Foo {
            a: int,
            b: float,
            c: float,
        }

        pub fn foo_new(a: int, b: float, c: float) -> Foo {
            Foo { a, b, c }
        }
    "#,
    );

    let a = 5i64;
    let b = 1.0f64;
    let c = 3.0f64;
    let foo: StructRef = invoke_fn!(driver.runtime_mut(), "foo_new", a, b, c).unwrap();

    driver.update(
        r#"
        struct Foo {
            a: int,
        }
    "#,
    );
    assert_eq!(foo.get::<i64>("a").unwrap(), a);
}

#[test]
fn map_struct_swap_fields1() {
    let mut driver = TestDriver::new(
        r#"
        struct Foo {
            a: float,
            b: int,
            c: float,
        }

        pub fn foo_new(a: float, b: int, c: float) -> Foo {
            Foo { a, b, c }
        }
    "#,
    );

    let a = 1.0f64;
    let b = 3i64;
    let c = 5.0f64;
    let foo: StructRef = invoke_fn!(driver.runtime_mut(), "foo_new", a, b, c).unwrap();

    driver.update(
        r#"
        struct Foo {
            c: float,
            a: float,
            b: int,
        }
    "#,
    );
    assert_eq!(foo.get::<f64>("a").unwrap(), a);
    assert_eq!(foo.get::<i64>("b").unwrap(), b);
    assert_eq!(foo.get::<f64>("c").unwrap(), c);
}

#[test]
fn map_struct_swap_fields2() {
    let mut driver = TestDriver::new(
        r#"
        struct Foo {
            a: float,
            b: int,
            c: float,
            d: int,
        }

        pub fn foo_new(a: float, b: int, c: float, d: int) -> Foo {
            Foo { a, b, c, d }
        }
    "#,
    );

    let a = 1.0f64;
    let b = 3i64;
    let c = 5.0f64;
    let d = 7i64;
    let foo: StructRef = invoke_fn!(driver.runtime_mut(), "foo_new", a, b, c, d).unwrap();

    driver.update(
        r#"
        struct Foo {
            d: int,
            c: float,
            b: int,
            a: float,
        }
    "#,
    );
    assert_eq!(foo.get::<f64>("a").unwrap(), a);
    assert_eq!(foo.get::<i64>("b").unwrap(), b);
    assert_eq!(foo.get::<f64>("c").unwrap(), c);
    assert_eq!(foo.get::<i64>("d").unwrap(), d);
}

#[test]
fn map_struct_rename_field1() {
    let mut driver = TestDriver::new(
        r#"
        struct Foo {
            a: int,
            b: float,
            c: float,
        }

        pub fn foo_new(a: int, b: float, c: float) -> Foo {
            Foo { a, b, c }
        }
    "#,
    );

    let a = 5i64;
    let b = 1.0f64;
    let c = 3.0f64;
    let foo: StructRef = invoke_fn!(driver.runtime_mut(), "foo_new", a, b, c).unwrap();

    driver.update(
        r#"
        struct Foo {
            a: int,
            d: float,
            c: float,
        }
    "#,
    );
    assert_eq!(foo.get::<i64>("a").unwrap(), a);
    assert_eq!(foo.get::<f64>("d").unwrap(), b);
    assert_eq!(foo.get::<f64>("c").unwrap(), c);
}

#[test]
fn map_struct_rename_field2() {
    let mut driver = TestDriver::new(
        r#"
        struct Foo {
            a: int,
            b: float,
            c: float,
        }

        pub fn foo_new(a: int, b: float, c: float) -> Foo {
            Foo { a, b, c }
        }
    "#,
    );

    let a = 5i64;
    let b = 1.0f64;
    let c = 3.0f64;
    let foo: StructRef = invoke_fn!(driver.runtime_mut(), "foo_new", a, b, c).unwrap();

    driver.update(
        r#"
        struct Foo {
            d: int,
            e: float,
            f: float,
        }
    "#,
    );
    assert_eq!(foo.get::<i64>("d").unwrap(), a);
    assert_eq!(foo.get::<f64>("e").unwrap(), b);
    assert_eq!(foo.get::<f64>("f").unwrap(), c);
}

#[test]
fn map_struct_all() {
    let mut driver = TestDriver::new(
        r#"
        struct Foo {
            a: int,
            b: float,
            c: float,
        }

        pub fn foo_new(a: int, b: float, c: float) -> Foo {
            Foo { a, b, c }
        }
    "#,
    );

    let a = 5i64;
    let b = 1.0f64;
    let c = 3.0f64;
    let foo: StructRef = invoke_fn!(driver.runtime_mut(), "foo_new", a, b, c).unwrap();

    driver.update(
        r#"
        struct Foo {
            b: float,   // move
            d: int,     // move + rename
        //  c: float,   // remove    
            e: int,     // add
        }
    "#,
    );
    assert_eq!(foo.get::<f64>("b").unwrap(), b);
    assert_eq!(foo.get::<i64>("d").unwrap(), a);
    assert_eq!(foo.get::<i64>("e").unwrap(), 0);
}

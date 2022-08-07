use mun_runtime::{ArrayRef, StructRef};
use mun_test::CompileAndRunTestDriver;
use std::sync::Arc;

#[macro_use]
mod util;

#[test]
fn gc_trace() {
    let driver = CompileAndRunTestDriver::new(
        r#"
    pub struct Foo {
        quz: f64,
        bar: Bar,
    }

    pub struct Bar {
        baz: i64
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
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let runtime = &driver.runtime;
    let value: StructRef = runtime.invoke("new_foo", ()).unwrap();
    let value = value.root();

    assert_eq!(runtime.gc_collect(), false);
    assert!(runtime.gc_stats().allocated_memory > 0);

    drop(value);

    assert_eq!(runtime.gc_collect(), true);
    assert_eq!(runtime.gc_stats().allocated_memory, 0);
}

#[test]
fn map_struct_insert_field1() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo {
            b: i64,
            c: f64,
        }

        pub fn foo_new(b: i64, c: f64) -> Foo {
            Foo { b, c }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let b = 5i64;
    let c = 3.0f64;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (b, c)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        struct Foo {
            a: i64,
            b: i64,
            c: f64,
        }
    "#,
    );

    let runtime = &driver.runtime;
    assert_eq!(foo_struct.as_ref(runtime).get::<i64>("a").unwrap(), 0);
    assert_eq!(foo_struct.as_ref(runtime).get::<i64>("b").unwrap(), b);
    assert_eq!(foo_struct.as_ref(runtime).get::<f64>("c").unwrap(), c);
}

#[test]
fn map_struct_insert_field2() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo {
            a: i64,
            c: f64,
        }

        pub fn foo_new(a: i64, c: f64) -> Foo {
            Foo { a, c }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i64;
    let c = 3.0f64;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, c)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        struct Foo {
            a: i64,
            b: f64,
            c: f64,
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i64>("a").unwrap(),
        a
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("b").unwrap(),
        0.0
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("c").unwrap(),
        c
    );
}

#[test]
fn map_struct_insert_field3() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo {
            a: i64,
            b: f64,
        }

        pub fn foo_new(a: i64, b: f64) -> Foo {
            Foo { a, b }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i64;
    let b = 3.0f64;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        struct Foo {
            a: i64,
            b: f64,
            c: f64,
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i64>("a").unwrap(),
        a
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("b").unwrap(),
        b
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("c").unwrap(),
        0.0
    );
}

#[test]
fn map_struct_remove_field1() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo {
            a: f64,
            b: f64,
            c: i64,
        }

        pub fn foo_new(a: f64, b: f64, c: i64) -> Foo {
            Foo { a, b, c }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 1.0f64;
    let b = 3.0f64;
    let c = 5i64;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, c)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        struct Foo {
            c: i64,
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i64>("c").unwrap(),
        c
    );
}

#[test]
fn map_struct_remove_field2() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo {
            a: f64,
            b: i64,
            c: f64,
        }

        pub fn foo_new(a: f64, b: i64, c: f64) -> Foo {
            Foo { a, b, c }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 1.0f64;
    let b = 5i64;
    let c = 3.0f64;
    let foo: StructRef = driver.runtime.invoke("foo_new", (a, b, c)).unwrap();
    let foo = foo.root();

    driver.update(
        "mod.mun",
        r#"
        struct Foo {
            b: i64,
        }
    "#,
    );
    assert_eq!(foo.as_ref(&driver.runtime).get::<i64>("b").unwrap(), b);
}

#[test]
fn map_struct_remove_field3() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo {
            a: i64,
            b: f64,
            c: f64,
        }

        pub fn foo_new(a: i64, b: f64, c: f64) -> Foo {
            Foo { a, b, c }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i64;
    let b = 1.0f64;
    let c = 3.0f64;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, c)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        struct Foo {
            a: i64,
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i64>("a").unwrap(),
        a
    );
}

#[test]
fn map_struct_cast_fields1() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo(
            u8,
            i16,
            u32,
            i64,
            f32,
        )

        pub fn foo_new(a: u8, b: i16, c: u32, d: i64, e: f32) -> Foo {
            Foo(a, b, c, d, e)
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 1u8;
    let b = -2i16;
    let c = 3u32;
    let d = -4i64;
    let e = 3.14f32;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, c, d, e)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        struct Foo(
            u16,
            i32,
            u64,
            i128,
            f64,
        )
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<u16>("0").unwrap(),
        a.into()
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i32>("1").unwrap(),
        b.into()
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<u64>("2").unwrap(),
        c.into()
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i128>("3").unwrap(),
        d.into()
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("4").unwrap(),
        e.into()
    );
}

#[test]
fn map_struct_cast_fields2() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo(
            i16,
        )

        pub fn foo_new(a: i16) -> Foo {
            Foo(a)
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = -2i16;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a,)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        struct Foo(
            u16,    // Cannot convert from `i16` to `u16`
        )
    "#,
    );

    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<u16>("0").unwrap(),
        0
    );
}

#[test]
fn map_struct_swap_fields1() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo {
            a: f64,
            b: i64,
            c: f64,
        }

        pub fn foo_new(a: f64, b: i64, c: f64) -> Foo {
            Foo { a, b, c }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 1.0f64;
    let b = 3i64;
    let c = 5.0f64;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, c)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        struct Foo {
            c: f64,
            a: f64,
            b: i64,
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("a").unwrap(),
        a
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i64>("b").unwrap(),
        b
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("c").unwrap(),
        c
    );
}

#[test]
fn map_struct_swap_fields2() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo {
            a: f64,
            b: i64,
            c: f64,
            d: i64,
        }

        pub fn foo_new(a: f64, b: i64, c: f64, d: i64) -> Foo {
            Foo { a, b, c, d }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 1.0f64;
    let b = 3i64;
    let c = 5.0f64;
    let d = 7i64;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, c, d)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        struct Foo {
            d: i64,
            c: f64,
            b: i64,
            a: f64,
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("a").unwrap(),
        a
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i64>("b").unwrap(),
        b
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("c").unwrap(),
        c
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i64>("d").unwrap(),
        d
    );
}

#[test]
fn map_struct_rename_field1() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo {
            a: i64,
            b: f64,
            c: f64,
        }

        pub fn foo_new(a: i64, b: f64, c: f64) -> Foo {
            Foo { a, b, c }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i64;
    let b = 1.0f64;
    let c = 3.0f64;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, c)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        struct Foo {
            a: i64,
            d: f64,
            c: f64,
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i64>("a").unwrap(),
        a
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("d").unwrap(),
        b
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("c").unwrap(),
        c
    );
}

#[test]
fn map_struct_rename_field2() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo {
            a: i64,
            b: f64,
            c: f64,
        }

        pub fn foo_new(a: i64, b: f64, c: f64) -> Foo {
            Foo { a, b, c }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i64;
    let b = 1.0f64;
    let c = 3.0f64;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, c)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        struct Foo {
            d: i64,
            e: f64,
            f: f64,
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i64>("d").unwrap(),
        a
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("e").unwrap(),
        b
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("f").unwrap(),
        c
    );
}

#[test]
fn map_struct_all() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo {
            a: i32,
            b: f64,
            c: f64,
            d: i32,
        }

        pub fn foo_new(a: i32, b: f64, c: f64, d: i32) -> Foo {
            Foo { a, b, c, d }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i32;
    let b = 1.0f64;
    let c = 3.0f64;
    let d = -1i32;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, c, d)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        pub struct Foo {
            b: f64, // move
        //  c: f64, // remove
            d: i64, // move + convert
            e: i32, // move + rename
            f: i32, // add
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("b").unwrap(),
        b
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i64>("d").unwrap(),
        d.into()
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i32>("e").unwrap(),
        a
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i32>("f").unwrap(),
        0
    );
}

#[test]
fn map_array_to_array_different_array_to_primitive_different() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo {
            a: i32,
            b: [[i32]],
            c: f32,
        }

        pub fn foo_new(a: i32, b: i32, c: f32) -> Foo {
            Foo { a, b: [[b], [a], [b]], c }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i32;
    let b = 1i32;
    let c = 3.0f32;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, c)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        pub struct Foo {
            a: i32,
            b: [i64],
            c: f32,
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i32>("a").unwrap(),
        a
    );

    let b_array = foo_struct
        .as_ref(&driver.runtime)
        .get::<ArrayRef<'_, i64>>("b")
        .unwrap();

    assert_eq!(b_array.iter().count(), 3);

    b_array
        .iter()
        .zip([b, a, b].into_iter())
        .for_each(|(lhs, rhs)| {
            assert_eq!(lhs, rhs.into());
        });

    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f32>("c").unwrap(),
        c
    );
}

#[test]
fn map_array_to_array_different_array_to_primitive_same() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo {
            a: i32,
            b: [[i32]],
            c: f32,
        }

        pub fn foo_new(a: i32, b: i32, c: f32) -> Foo {
            Foo { a, b: [[b], [a], [b]], c }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i32;
    let b = 1i32;
    let c = 3.0f32;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, c)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        pub struct Foo {
            a: i32,
            b: [i32],
            c: f32,
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i32>("a").unwrap(),
        a
    );

    let b_array = foo_struct
        .as_ref(&driver.runtime)
        .get::<ArrayRef<'_, i32>>("b")
        .unwrap();

    assert_eq!(b_array.iter().count(), 3);

    b_array
        .iter()
        .zip([b, a, b].into_iter())
        .for_each(|(lhs, rhs)| {
            assert_eq!(lhs, rhs);
        });

    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f32>("c").unwrap(),
        c
    );
}

#[test]
fn map_array_to_array_different_array_to_struct_different() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct(gc) Bar(i32);
        pub struct(value) Baz(i32);

        pub struct Foo {
            a: i32,
            b: [[Bar]],
            c: [[Baz]],
            d: f32,
        }

        pub fn foo_new(a: i32, b: i32, d: f32) -> Foo {
            Foo {
                a,
                b: [[Bar(b)], [Bar(a)], [Bar(b)]],
                c: [[Baz(b)], [Baz(a)], [Baz(b)]],
                d
            }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i32;
    let b = 1i32;
    let d = 3.0f32;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, d)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        pub struct(gc) Bar(i64);
        pub struct(value) Baz(i64);

        pub struct Foo {
            a: i32,
            b: [Bar],
            c: [Baz],
            d: f32,
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i32>("a").unwrap(),
        a
    );

    for field_name in ["b", "c"] {
        let array = foo_struct
            .as_ref(&driver.runtime)
            .get::<ArrayRef<'_, StructRef>>(field_name)
            .unwrap();

        assert_eq!(array.iter().count(), 3);

        array
            .iter()
            .zip([b, a, b].into_iter())
            .for_each(|(lhs, rhs)| {
                assert_eq!(lhs.get::<i64>("0").unwrap(), rhs.into());
            });
    }

    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f32>("d").unwrap(),
        d
    );
}

#[test]
fn map_array_to_array_different_array_to_struct_same() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Bar(i32);
        pub struct(value) Baz(i32);

        pub struct Foo {
            a: i32,
            b: [[Bar]],
            c: [[Baz]],
            d: f32,
        }

        pub fn foo_new(a: i32, b: i32, d: f32) -> Foo {
            Foo {
                a,
                b: [[Bar(b)], [Bar(a)], [Bar(b)]],
                c: [[Baz(b)], [Baz(a)], [Baz(b)]],
                d
            }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i32;
    let b = 1i32;
    let d = 3.0f32;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, d)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        pub struct Bar(i32);
        pub struct(value) Baz(i32);

        pub struct Foo {
            a: i32,
            b: [Bar],
            c: [Baz],
            d: f32,
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i32>("a").unwrap(),
        a
    );

    for field_name in ["b", "c"] {
        let array = foo_struct
            .as_ref(&driver.runtime)
            .get::<ArrayRef<'_, StructRef>>(field_name)
            .unwrap();

        assert_eq!(array.iter().count(), 3);

        array
            .iter()
            .zip([b, a, b].into_iter())
            .for_each(|(lhs, rhs)| {
                assert_eq!(lhs.get::<i32>("0").unwrap(), rhs);
            });
    }

    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f32>("d").unwrap(),
        d
    );
}

#[test]
fn map_array_to_array_different_primitive_to_array_different() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo {
            a: i32,
            b: [i32],
            c: f32,
        }

        pub fn foo_new(a: i32, b: i32, c: f32) -> Foo {
            Foo { a, b: [b, a, b], c }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i32;
    let b = 1i32;
    let c = 3.0f32;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, c)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        pub struct Foo {
            a: i32,
            b: [[i64]],
            c: f32,
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i32>("a").unwrap(),
        a
    );

    let b_array = foo_struct
        .as_ref(&driver.runtime)
        .get::<ArrayRef<'_, ArrayRef<'_, i64>>>("b")
        .unwrap();

    assert_eq!(b_array.iter().count(), 3);

    b_array
        .iter()
        .zip([b, a, b].into_iter())
        .for_each(|(lhs, rhs)| {
            assert_eq!(lhs.iter().count(), 1);
            assert_eq!(
                lhs.iter().next().expect("Array must have a value."),
                rhs.into()
            );
        });

    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f32>("c").unwrap(),
        c
    );
}

#[test]
fn map_array_to_array_different_primitive_to_array_same() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo {
            a: i32,
            b: [i32],
            c: f32,
        }

        pub fn foo_new(a: i32, b: i32, c: f32) -> Foo {
            Foo { a, b: [b, a, b], c }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i32;
    let b = 1i32;
    let c = 3.0f32;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, c)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        pub struct Foo {
            a: i32,
            b: [[i32]],
            c: f32,
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i32>("a").unwrap(),
        a
    );

    let b_array = foo_struct
        .as_ref(&driver.runtime)
        .get::<ArrayRef<'_, ArrayRef<'_, i32>>>("b")
        .unwrap();

    assert_eq!(b_array.iter().count(), 3);

    b_array
        .iter()
        .zip([b, a, b].into_iter())
        .for_each(|(lhs, rhs)| {
            assert_eq!(lhs.iter().count(), 1);
            assert_eq!(lhs.iter().next().expect("Array must have a value."), rhs);
        });

    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f32>("c").unwrap(),
        c
    );
}

#[test]
fn map_array_to_array_different_primitive_to_primitive() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo {
            a: i32,
            b: [i32],
            c: f32,
        }

        pub fn foo_new(a: i32, b: i32, c: f32) -> Foo {
            Foo { a, b: [b, a, b], c }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i32;
    let b = 1i32;
    let c = 3.0f32;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, c)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        pub struct Foo {
            a: i32,
            b: [i64],
            c: f32,
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i32>("a").unwrap(),
        a
    );

    let b_array = foo_struct
        .as_ref(&driver.runtime)
        .get::<ArrayRef<'_, i64>>("b")
        .unwrap();

    assert_eq!(b_array.iter().count(), 3);

    b_array
        .iter()
        .zip([b, a, b].into_iter())
        .for_each(|(lhs, rhs)| {
            assert_eq!(lhs, rhs.into());
        });

    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f32>("c").unwrap(),
        c
    );
}

#[test]
fn map_array_to_array_different_primitive_to_struct() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo {
            a: i32,
            b: [i32],
            c: [i32],
            d: f32,
        }

        pub fn foo_new(a: i32, b: i32, d: f32) -> Foo {
            Foo { a, b: [b, a, b], c: [b, a, b], d }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i32;
    let b = 1i32;
    let d = 3.0f32;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, d)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        pub struct(gc) Bar(i64);
        pub struct(value) Baz(i64);

        pub struct Foo {
            a: i32,
            b: [Bar],
            c: [Baz],
            d: f32,
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i32>("a").unwrap(),
        a
    );

    for field_name in ["b", "c"] {
        let array = foo_struct
            .as_ref(&driver.runtime)
            .get::<ArrayRef<'_, StructRef>>(field_name)
            .unwrap();

        assert_eq!(array.iter().count(), 3);

        array.iter().for_each(|s| {
            assert_eq!(s.get::<i64>("0").unwrap(), 0);
        });
    }

    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f32>("d").unwrap(),
        d
    );
}

#[test]
fn map_array_to_array_different_struct_to_array_different() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Bar(i32);
        pub struct(value) Baz(i32);

        pub struct Foo {
            a: i32,
            b: [Bar],
            c: [Baz],
            d: f32,
        }

        pub fn foo_new(a: i32, b: i32, d: f32) -> Foo {
            Foo {
                a,
                b: [Bar(b), Bar(a), Bar(b)],
                c: [Baz(b), Baz(a), Baz(b)],
                d
            }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i32;
    let b = 1i32;
    let d = 3.0f32;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, d)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        pub struct Bar(i64);
        pub struct(value) Baz(i64);

        pub struct Foo {
            a: i32,
            b: [[Bar]],
            c: [[Baz]],
            d: f32,
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i32>("a").unwrap(),
        a
    );

    for field_name in ["b", "c"] {
        let array = foo_struct
            .as_ref(&driver.runtime)
            .get::<ArrayRef<'_, ArrayRef<'_, StructRef>>>(field_name)
            .unwrap();

        assert_eq!(array.iter().count(), 3);

        array
            .iter()
            .zip([b, a, b].into_iter())
            .for_each(|(lhs, rhs)| {
                assert_eq!(lhs.iter().count(), 1);

                assert_eq!(
                    lhs.iter()
                        .next()
                        .expect("Array must have a value.")
                        .get::<i64>("0")
                        .unwrap(),
                    rhs.into()
                );
            });
    }

    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f32>("d").unwrap(),
        d
    );
}

#[test]
fn map_array_to_array_different_struct_to_array_same() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Bar(i32);
        pub struct(value) Baz(i32);

        pub struct Foo {
            a: i32,
            b: [Bar],
            c: [Baz],
            d: f32,
        }

        pub fn foo_new(a: i32, b: i32, d: f32) -> Foo {
            Foo {
                a,
                b: [Bar(b), Bar(a), Bar(b)],
                c: [Baz(b), Baz(a), Baz(b)],
                d
            }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i32;
    let b = 1i32;
    let d = 3.0f32;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, d)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        pub struct Bar(i32);
        pub struct(value) Baz(i32);

        pub struct Foo {
            a: i32,
            b: [[Bar]],
            c: [[Baz]],
            d: f32,
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i32>("a").unwrap(),
        a
    );

    for field_name in ["b", "c"] {
        let array = foo_struct
            .as_ref(&driver.runtime)
            .get::<ArrayRef<'_, ArrayRef<'_, StructRef>>>(field_name)
            .unwrap();

        assert_eq!(array.iter().count(), 3);

        array
            .iter()
            .zip([b, a, b].into_iter())
            .for_each(|(lhs, rhs)| {
                assert_eq!(lhs.iter().count(), 1);

                assert_eq!(
                    lhs.iter()
                        .next()
                        .expect("Array must have a value.")
                        .get::<i32>("0")
                        .unwrap(),
                    rhs
                );
            });
    }

    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f32>("d").unwrap(),
        d
    );
}

#[test]
fn map_array_to_array_different_struct_to_struct() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct(gc) Bar(i32);
        pub struct(value) Baz(i32);

        pub struct Foo {
            a: i32,
            b: [Bar],
            c: [Baz],
            d: f32,
        }

        pub fn foo_new(a: i32, b: i32, d: f32) -> Foo {
            Foo {
                a,
                b: [Bar(b), Bar(a), Bar(b)],
                c: [Baz(b), Baz(a), Baz(b)],
                d
            }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i32;
    let b = 1i32;
    let d = 3.0f32;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, d)).unwrap();
    let foo_struct = foo_struct.root();

    println!(
        "b before: {:?}",
        foo_struct
            .as_ref(&driver.runtime)
            .get::<ArrayRef<'_, StructRef>>("b")
            .unwrap()
            .type_info()
    );
    println!(
        "c before: {:?}",
        foo_struct
            .as_ref(&driver.runtime)
            .get::<ArrayRef<'_, StructRef>>("c")
            .unwrap()
            .type_info()
    );

    driver.update(
        "mod.mun",
        r#"
        pub struct(gc) Bar(i64);
        pub struct(value) Baz(i64);

        pub struct Foo {
            a: i32,
            b: [Bar],
            c: [Baz],
            d: f32,
        }
    "#,
    );
    println!(
        "b after: {:?}",
        foo_struct
            .as_ref(&driver.runtime)
            .get::<ArrayRef<'_, StructRef>>("b")
            .unwrap()
            .type_info()
    );
    println!(
        "c after: {:?}",
        foo_struct
            .as_ref(&driver.runtime)
            .get::<ArrayRef<'_, StructRef>>("c")
            .unwrap()
            .type_info()
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i32>("a").unwrap(),
        a
    );

    for field_name in ["b", "c"] {
        let array = foo_struct
            .as_ref(&driver.runtime)
            .get::<ArrayRef<'_, StructRef>>(field_name)
            .unwrap();

        assert_eq!(array.iter().count(), 3);

        array
            .iter()
            .zip([b, a, b].into_iter())
            .for_each(|(lhs, rhs)| {
                // println!("struct type: {:?}", lhs.type_info());
                assert_eq!(lhs.get::<i64>("0").unwrap(), rhs.into());
            });
    }

    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f32>("d").unwrap(),
        d
    );
}

#[test]
fn map_array_to_array_same_primitive() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo {
            a: i32,
            b: [f64],
            c: f64,
        }

        pub fn foo_new(a: i32, b: f64, c: f64) -> Foo {
            Foo { a, b: [b], c }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i32;
    let b = 1.0f64;
    let c = 3.0f64;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, c)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        pub struct Foo {
            a: i64,
            b: [f64],
            c: f64,
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i64>("a").unwrap(),
        i64::from(a)
    );

    let b_array = foo_struct
        .as_ref(&driver.runtime)
        .get::<ArrayRef<'_, f64>>("b")
        .unwrap();

    assert_eq!(b_array.iter().count(), 1);
    assert_eq!(b_array.iter().next().expect("Array must have a value."), b);

    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("c").unwrap(),
        c
    );
}

#[test]
fn map_array_to_array_same_struct() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Bar(f64);

        pub struct Foo {
            a: i32,
            b: [Bar],
            c: f64,
        }

        pub fn foo_new(a: i32, b: f64, c: f64) -> Foo {
            Foo { a, b: [Bar(b)], c }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i32;
    let b = 1.0f64;
    let c = 3.0f64;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, c)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        pub struct Bar(f64);

        pub struct Foo {
            a: i64,
            b: [Bar],
            c: f64,
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i64>("a").unwrap(),
        i64::from(a)
    );

    let b_array = foo_struct
        .as_ref(&driver.runtime)
        .get::<ArrayRef<'_, StructRef>>("b")
        .unwrap();

    assert_eq!(b_array.iter().count(), 1);
    assert_eq!(
        b_array
            .iter()
            .next()
            .expect("Array must have a value.")
            .get::<f64>("0")
            .unwrap(),
        b
    );

    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("c").unwrap(),
        c
    );
}

#[test]
fn map_array_to_primitive_different() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo {
            a: i32,
            b: [f64],
            c: f64,
        }

        pub fn foo_new(a: i32, b: f64, c: f64) -> Foo {
            Foo { a, b: [b], c }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i32;
    let b = 1.0f64;
    let c = 3.0f64;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, c)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        pub struct Foo {
            a: i32,
            b: i64,
            c: f64,
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i32>("a").unwrap(),
        a
    );

    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i64>("b").unwrap(),
        0
    );

    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("c").unwrap(),
        c
    );
}

#[test]
fn map_array_to_primitive_same() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo {
            a: i32,
            b: [f64],
            c: f64,
        }

        pub fn foo_new(a: i32, b: f64, c: f64) -> Foo {
            Foo { a, b: [b], c }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i32;
    let b = 1.0f64;
    let c = 3.0f64;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, c)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        pub struct Foo {
            a: i32,
            b: f64,
            c: f64,
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i32>("a").unwrap(),
        a
    );

    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("b").unwrap(),
        b
    );

    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("c").unwrap(),
        c
    );
}

#[test]
fn map_array_to_struct_different() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct(gc) Bar(f32);
        pub struct(value) Baz(i32);

        pub struct Foo {
            a: i32,
            b: [Bar],
            c: [Baz],
            d: f64,
        }

        pub fn foo_new(a: i32, b: f32, c: i32, d: f64) -> Foo {
            Foo { a, b: [Bar(b)], c: [Baz(c)], d }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i32;
    let b = 1.0f32;
    let c = -1i32;
    let d = 3.0f64;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, c, d)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        pub struct(gc) Bar(f64);
        pub struct(value) Baz(i64);

        pub struct Foo {
            a: i32,
            b: Bar,
            c: Baz,
            d: f64,
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i32>("a").unwrap(),
        a
    );

    let bar_struct = foo_struct
        .as_ref(&driver.runtime)
        .get::<StructRef>("b")
        .unwrap();

    assert_eq!(bar_struct.get::<f64>("0").unwrap(), b.into());

    let baz_struct = foo_struct
        .as_ref(&driver.runtime)
        .get::<StructRef>("c")
        .unwrap();

    assert_eq!(baz_struct.get::<i64>("0").unwrap(), c.into());

    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("d").unwrap(),
        d
    );
}

#[test]
fn map_array_to_struct_same() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct(gc) Bar(f32);
        pub struct(value) Baz(i32);

        pub struct Foo {
            a: i32,
            b: [Bar],
            c: [Baz],
            d: f64,
        }

        pub fn foo_new(a: i32, b: f32, c: i32, d: f64) -> Foo {
            Foo { a, b: [Bar(b)], c: [Baz(c)], d }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i32;
    let b = 1.0f32;
    let c = -1i32;
    let d = 3.0f64;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, c, d)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        pub struct(gc) Bar(f32);
        pub struct(value) Baz(i32);

        pub struct Foo {
            a: i32,
            b: Bar,
            c: Baz,
            d: f64,
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i32>("a").unwrap(),
        a
    );

    let bar_struct = foo_struct
        .as_ref(&driver.runtime)
        .get::<StructRef>("b")
        .unwrap();

    assert_eq!(bar_struct.get::<f32>("0").unwrap(), b);

    let baz_struct = foo_struct
        .as_ref(&driver.runtime)
        .get::<StructRef>("c")
        .unwrap();

    assert_eq!(baz_struct.get::<i32>("0").unwrap(), c);

    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("d").unwrap(),
        d
    );
}

#[test]
fn map_primitive_to_array_same() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo {
            a: i32,
            b: f64,
            c: f64,
            d: i32,
        }

        pub fn foo_new(a: i32, b: f64, c: f64, d: i32) -> Foo {
            Foo { a, b, c, d }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i32;
    let b = 1.0f64;
    let c = 3.0f64;
    let d = -1i32;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, c, d)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        pub struct Foo {
            a: i32,
            b: [f64],
            c: f64,
            d: [i32],
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i32>("a").unwrap(),
        a
    );

    let b_array = foo_struct
        .as_ref(&driver.runtime)
        .get::<ArrayRef<'_, f64>>("b")
        .unwrap();

    assert_eq!(b_array.iter().count(), 1);
    assert_eq!(b_array.iter().next().expect("Array must have a value."), b);

    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("c").unwrap(),
        c
    );

    let d_array = foo_struct
        .as_ref(&driver.runtime)
        .get::<ArrayRef<'_, i32>>("d")
        .unwrap();

    assert_eq!(d_array.iter().count(), 1);
    assert_eq!(d_array.iter().next().expect("Array must have a value."), d);
}

#[test]
fn map_primitive_to_array_different() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo {
            a: i32,
            b: f32,
            c: f64,
            d: i32,
        }

        pub fn foo_new(a: i32, b: f32, c: f64, d: i32) -> Foo {
            Foo { a, b, c, d }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i32;
    let b = 1.0f32;
    let c = 3.0f64;
    let d = -1i32;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, c, d)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        pub struct Foo {
            a: i32,
            b: [f64],
            c: f64,
            d: [i64],
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i32>("a").unwrap(),
        a
    );

    let b_array = foo_struct
        .as_ref(&driver.runtime)
        .get::<ArrayRef<'_, f64>>("b")
        .unwrap();

    assert_eq!(b_array.iter().count(), 1);
    assert_eq!(
        b_array.iter().next().expect("Array must have a value."),
        b.into()
    );

    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("c").unwrap(),
        c
    );

    let d_array = foo_struct
        .as_ref(&driver.runtime)
        .get::<ArrayRef<'_, i64>>("d")
        .unwrap();

    assert_eq!(d_array.iter().count(), 1);
    assert_eq!(
        d_array.iter().next().expect("Array must have a value."),
        d.into()
    );
}

#[test]
fn map_struct_to_array_same() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Bar(f64);

        pub struct Foo {
            a: i32,
            b: Bar,
            c: f64,
        }

        pub fn foo_new(a: i32, b: f64, c: f64) -> Foo {
            Foo { a, b: Bar(b), c }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i32;
    let b = 1.0f64;
    let c = 3.0f64;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, c)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        pub struct Bar(f64);

        pub struct Foo {
            a: i32,
            b: [Bar],
            c: f64,
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i32>("a").unwrap(),
        a
    );

    let b_array = foo_struct
        .as_ref(&driver.runtime)
        .get::<ArrayRef<'_, StructRef>>("b")
        .unwrap();

    assert_eq!(b_array.iter().count(), 1);
    assert_eq!(
        b_array
            .iter()
            .next()
            .expect("Array must have a value.")
            .get::<f64>("0")
            .unwrap(),
        b
    );

    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("c").unwrap(),
        c
    );
}

#[test]
fn map_struct_to_array_different() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Bar(f32);

        pub struct Foo {
            a: i32,
            b: Bar,
            c: f64,
        }

        pub fn foo_new(a: i32, b: f32, c: f64) -> Foo {
            Foo { a, b: Bar(b), c }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i32;
    let b = 1.0f32;
    let c = 3.0f64;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, c)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        pub struct Bar(f64);

        pub struct Foo {
            a: i32,
            b: [Bar],
            c: f64,
        }
    "#,
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i32>("a").unwrap(),
        a
    );

    let b_array = foo_struct
        .as_ref(&driver.runtime)
        .get::<ArrayRef<'_, StructRef>>("b")
        .unwrap();

    assert_eq!(b_array.iter().count(), 1);
    assert_eq!(
        b_array
            .iter()
            .next()
            .expect("Array must have a value.")
            .get::<f64>("0")
            .unwrap(),
        b.into()
    );

    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("c").unwrap(),
        c
    );
}

#[test]
fn insert_array() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo {
            b: i64,
            c: f64,
        }

        pub fn foo_new(b: i64, c: f64) -> Foo {
            Foo { b, c }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let b = 5i64;
    let c = 3.0f64;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (b, c)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        struct Foo {
            a: [i64],
            b: i64,
            c: f64,
        }
    "#,
    );

    let runtime = &driver.runtime;

    let a_array = foo_struct
        .as_ref(&driver.runtime)
        .get::<ArrayRef<'_, i64>>("a")
        .unwrap();

    assert_eq!(a_array.iter().count(), 0);

    assert_eq!(foo_struct.as_ref(runtime).get::<i64>("b").unwrap(), b);
    assert_eq!(foo_struct.as_ref(runtime).get::<f64>("c").unwrap(), c);
}

#[test]
fn delete_used_struct() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo {
            a: i64,
            b: f64,
            c: f64,
        }

        pub fn foo_new(a: i64, b: f64, c: f64) -> Foo {
            Foo { a, b, c }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i64;
    let b = 1.0f64;
    let c = 3.0f64;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, b, c)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        pub struct Bar(i64);

        pub fn bar_new(a: i64) -> Bar {
            Bar(a)
        }
    "#,
    );

    assert!(driver.runtime.get_function_definition("foo_new").is_none());
    assert!(driver.runtime.get_function_definition("bar_new").is_some());
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i64>("a").unwrap(),
        a
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("b").unwrap(),
        b
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("c").unwrap(),
        c
    );
}

#[test]
fn nested_structs() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
    pub struct(gc) GcStruct(f32, f32);
    pub struct(value) ValueStruct(f32, f32);

    pub struct(gc) GcWrapper(GcStruct, ValueStruct)
    pub struct(value) ValueWrapper(GcStruct, ValueStruct);

    pub fn new_gc_struct(a: f32, b: f32) -> GcStruct {
        GcStruct(a, b)
    }

    pub fn new_value_struct(a: f32, b: f32) -> ValueStruct {
        ValueStruct(a, b)
    }

    pub fn new_gc_wrapper(a: GcStruct, b: ValueStruct) -> GcWrapper {
        GcWrapper(a, b)
    }

    pub fn new_value_wrapper(a: GcStruct, b: ValueStruct) -> ValueWrapper {
        ValueWrapper(a, b)
    }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = -3.14f32;
    let b = 6.18f32;
    let gc_struct: StructRef = driver.runtime.invoke("new_gc_struct", (a, b)).unwrap();
    let value_struct: StructRef = driver.runtime.invoke("new_value_struct", (a, b)).unwrap();

    let gc_wrapper: StructRef = driver
        .runtime
        .invoke("new_gc_wrapper", (gc_struct.clone(), value_struct.clone()))
        .unwrap();

    let value_wrapper: StructRef = driver
        .runtime
        .invoke(
            "new_value_wrapper",
            (gc_struct.clone(), value_struct.clone()),
        )
        .unwrap();

    let gc_wrapper = gc_wrapper.root();
    let value_wrapper = value_wrapper.root();

    // Tests mapping of `gc -> gc`, `value -> value`
    driver.update(
        "mod.mun",
        r#"
    pub struct(gc) GcStruct(f64, f64);
    pub struct(value) ValueStruct(f64, f64);

    pub struct(gc) GcWrapper(GcStruct, ValueStruct)
    pub struct(value) ValueWrapper(GcStruct, ValueStruct);
    "#,
    );

    let gc_0 = gc_wrapper
        .as_ref(&driver.runtime)
        .get::<StructRef>("0")
        .unwrap();
    assert_eq!(gc_0.get::<f64>("0"), Ok(a.into()));
    assert_eq!(gc_0.get::<f64>("1"), Ok(b.into()));

    let gc_1 = gc_wrapper
        .as_ref(&driver.runtime)
        .get::<StructRef>("1")
        .unwrap();
    assert_eq!(gc_1.get::<f64>("0"), Ok(a.into()));
    assert_eq!(gc_1.get::<f64>("1"), Ok(b.into()));

    let value_0 = value_wrapper
        .as_ref(&driver.runtime)
        .get::<StructRef>("0")
        .unwrap();
    assert_eq!(value_0.get::<f64>("0"), Ok(a.into()));
    assert_eq!(value_0.get::<f64>("1"), Ok(b.into()));

    let value_1 = value_wrapper
        .as_ref(&driver.runtime)
        .get::<StructRef>("1")
        .unwrap();
    assert_eq!(value_1.get::<f64>("0"), Ok(a.into()));
    assert_eq!(value_1.get::<f64>("1"), Ok(b.into()));

    // Tests an identity mapping
    driver.update(
        "mod.mun",
        r#"
    pub struct(gc) GcStruct(f64, f64);
    pub struct(value) ValueStruct(f64, f64);

    pub struct(gc) GcWrapper(GcStruct, ValueStruct)
    pub struct(value) ValueWrapper(GcStruct, ValueStruct);
    "#,
    );

    let gc_0 = gc_wrapper
        .as_ref(&driver.runtime)
        .get::<StructRef>("0")
        .unwrap();
    assert_eq!(gc_0.get::<f64>("0"), Ok(a.into()));
    assert_eq!(gc_0.get::<f64>("1"), Ok(b.into()));

    let gc_1 = gc_wrapper
        .as_ref(&driver.runtime)
        .get::<StructRef>("1")
        .unwrap();
    assert_eq!(gc_1.get::<f64>("0"), Ok(a.into()));
    assert_eq!(gc_1.get::<f64>("1"), Ok(b.into()));

    let value_0 = value_wrapper
        .as_ref(&driver.runtime)
        .get::<StructRef>("0")
        .unwrap();
    assert_eq!(value_0.get::<f64>("0"), Ok(a.into()));
    assert_eq!(value_0.get::<f64>("1"), Ok(b.into()));

    let value_1 = value_wrapper
        .as_ref(&driver.runtime)
        .get::<StructRef>("1")
        .unwrap();
    assert_eq!(value_1.get::<f64>("0"), Ok(a.into()));
    assert_eq!(value_1.get::<f64>("1"), Ok(b.into()));

    let gc_0 = gc_0.root();
    let gc_1 = gc_1.root();
    let value_0 = value_0.root();
    let value_1 = value_1.root();

    // Tests mapping of `gc -> value`, `value -> gc`
    driver.update(
        "mod.mun",
        r#"
    struct(value) GcStruct(f64, f64);
    struct(gc) ValueStruct(f64, f64);

    struct(gc) GcWrapper(GcStruct, ValueStruct)
    struct(value) ValueWrapper(GcStruct, ValueStruct);
    "#,
    );

    assert_eq!(gc_0.as_ref(&driver.runtime).get::<f64>("0"), Ok(a.into()));
    assert_eq!(gc_0.as_ref(&driver.runtime).get::<f64>("1"), Ok(b.into()));

    assert_eq!(gc_1.as_ref(&driver.runtime).get::<f64>("0"), Ok(a.into()));
    assert_eq!(gc_1.as_ref(&driver.runtime).get::<f64>("1"), Ok(b.into()));

    assert_eq!(
        value_0.as_ref(&driver.runtime).get::<f64>("0"),
        Ok(a.into())
    );
    assert_eq!(
        value_0.as_ref(&driver.runtime).get::<f64>("1"),
        Ok(b.into())
    );

    assert_eq!(
        value_1.as_ref(&driver.runtime).get::<f64>("0"),
        Ok(a.into())
    );
    assert_eq!(
        value_1.as_ref(&driver.runtime).get::<f64>("1"),
        Ok(b.into())
    );

    // Tests mapping of different struct type, when `gc -> value`, `value -> gc`, and
    // retention of an old library (due to removal of `GcStruct` and `ValueStruct`)
    driver.update(
        "mod.mun",
        r#"
    struct(gc) GcStruct2(f64);
    struct(value) ValueStruct2(f64);

    struct(gc) GcWrapper(GcStruct2, ValueStruct2)
    struct(value) ValueWrapper(GcStruct2, ValueStruct2);
    "#,
    );

    // Existing, rooted objects should remain untouched
    assert_eq!(gc_0.as_ref(&driver.runtime).get::<f64>("0"), Ok(a.into()));
    assert_eq!(gc_0.as_ref(&driver.runtime).get::<f64>("1"), Ok(b.into()));

    assert_eq!(gc_1.as_ref(&driver.runtime).get::<f64>("0"), Ok(a.into()));
    assert_eq!(gc_1.as_ref(&driver.runtime).get::<f64>("1"), Ok(b.into()));

    assert_eq!(
        value_0.as_ref(&driver.runtime).get::<f64>("0"),
        Ok(a.into())
    );
    assert_eq!(
        value_0.as_ref(&driver.runtime).get::<f64>("1"),
        Ok(b.into())
    );

    assert_eq!(
        value_1.as_ref(&driver.runtime).get::<f64>("0"),
        Ok(a.into())
    );
    assert_eq!(
        value_1.as_ref(&driver.runtime).get::<f64>("1"),
        Ok(b.into())
    );

    // The values in the wrappers should have been updated
    let mut gc_0 = gc_wrapper
        .as_ref(&driver.runtime)
        .get::<StructRef>("0")
        .unwrap();
    assert_eq!(gc_0.get::<f64>("0"), Ok(0.0));
    gc_0.set::<f64>("0", a.into()).unwrap();

    let mut gc_1 = gc_wrapper
        .as_ref(&driver.runtime)
        .get::<StructRef>("1")
        .unwrap();
    assert_eq!(gc_1.get::<f64>("0"), Ok(0.0));
    gc_1.set::<f64>("0", a.into()).unwrap();

    let mut value_0 = value_wrapper
        .as_ref(&driver.runtime)
        .get::<StructRef>("0")
        .unwrap();
    assert_eq!(value_0.get::<f64>("0"), Ok(0.0));
    value_0.set::<f64>("0", a.into()).unwrap();

    let mut value_1 = value_wrapper
        .as_ref(&driver.runtime)
        .get::<StructRef>("1")
        .unwrap();
    assert_eq!(value_1.get::<f64>("0"), Ok(0.0));
    value_1.set::<f64>("0", a.into()).unwrap();

    let gc_0 = gc_0.root();
    let gc_1 = gc_1.root();
    let value_0 = value_0.root();
    let value_1 = value_1.root();

    // Tests mapping of different struct type, when `gc -> gc`, `value -> value`
    driver.update(
        "mod.mun",
        r#"
    struct(gc) GcStruct(f64, f64);
    struct(value) ValueStruct(f64, f64);

    struct(gc) GcWrapper(GcStruct, ValueStruct)
    struct(value) ValueWrapper(GcStruct, ValueStruct);
    "#,
    );

    // Existing, rooted objects should remain untouched
    assert_eq!(gc_0.as_ref(&driver.runtime).get::<f64>("0"), Ok(a.into()));
    assert_eq!(gc_1.as_ref(&driver.runtime).get::<f64>("0"), Ok(a.into()));
    assert_eq!(
        value_0.as_ref(&driver.runtime).get::<f64>("0"),
        Ok(a.into())
    );
    assert_eq!(
        value_1.as_ref(&driver.runtime).get::<f64>("0"),
        Ok(a.into())
    );

    // The values in the wrappers should have been updated
    let gc_0 = gc_wrapper
        .as_ref(&driver.runtime)
        .get::<StructRef>("0")
        .unwrap();
    assert_eq!(gc_0.get::<f64>("0"), Ok(0.0));
    assert_eq!(gc_0.get::<f64>("1"), Ok(0.0));

    let gc_1 = gc_wrapper
        .as_ref(&driver.runtime)
        .get::<StructRef>("1")
        .unwrap();
    assert_eq!(gc_1.get::<f64>("0"), Ok(0.0));
    assert_eq!(gc_1.get::<f64>("1"), Ok(0.0));

    let value_0 = value_wrapper
        .as_ref(&driver.runtime)
        .get::<StructRef>("0")
        .unwrap();
    assert_eq!(value_0.get::<f64>("0"), Ok(0.0));
    assert_eq!(value_0.get::<f64>("1"), Ok(0.0));

    let value_1 = value_wrapper
        .as_ref(&driver.runtime)
        .get::<StructRef>("1")
        .unwrap();
    assert_eq!(value_1.get::<f64>("0"), Ok(0.0));
    assert_eq!(value_1.get::<f64>("1"), Ok(0.0));
}

#[test]
fn insert_struct() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
        pub struct Foo {
            a: i64,
            c: f64,
        }

        pub fn foo_new(a: i64, c: f64) -> Foo {
            Foo { a, c }
        }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a = 5i64;
    let c = 3.0f64;
    let foo_struct: StructRef = driver.runtime.invoke("foo_new", (a, c)).unwrap();
    let foo_struct = foo_struct.root();

    driver.update(
        "mod.mun",
        r#"
        struct Bar(i64);
        struct(value) Baz(f64);

        struct Foo {
            a: i64,
            b: Bar,
            c: f64,
            d: Baz,
        }
    "#,
    );

    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<i64>("a").unwrap(),
        a
    );
    assert_eq!(
        foo_struct.as_ref(&driver.runtime).get::<f64>("c").unwrap(),
        c
    );

    let b = foo_struct
        .as_ref(&driver.runtime)
        .get::<StructRef>("b")
        .unwrap();
    assert_eq!(b.get::<i64>("0"), Ok(0));

    let d = foo_struct
        .as_ref(&driver.runtime)
        .get::<StructRef>("d")
        .unwrap();
    assert_eq!(d.get::<f64>("0"), Ok(0.0));
}

#[test]
fn test_type_table() {
    let driver = CompileAndRunTestDriver::from_fixture(
        r#"
    //- /mun.toml
    [package]
    name="foo"
    version="0.0.0"

    //- /src/mod.mun

    //- /src/foo.mun
    use package::bar::Bar;

    pub struct Foo {
        bar: Bar
    }

    pub fn new_foo() -> Foo {
        Foo {
            bar: Bar {value: 3}
        }
    }

    //- /src/bar.mun
    pub struct Bar {
        value: i32
    }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let a: StructRef = driver
        .runtime
        .invoke("foo::new_foo", ())
        .expect("failed to call 'new_foo'");

    // Get the type of the Bar struct
    let bar_type = driver
        .runtime
        .get_type_info_by_name("bar::Bar")
        .expect("could not find Bar type");

    // Get the type of the `bar` field of `Foo`.
    let foo_bar_field_type = a
        .type_info()
        .as_struct()
        .expect("is not a struct?")
        .find_field_by_name("bar")
        .expect("could not find `bar` field")
        .type_info
        .clone();

    // These types should be equal
    assert_eq!(foo_bar_field_type, bar_type);

    // In fact the pointers should be equal!
    assert!(Arc::ptr_eq(&foo_bar_field_type, &bar_type));
}

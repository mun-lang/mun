use mun_runtime::{invoke_fn, StructRef};

#[macro_use]
mod util;

use util::*;

#[test]
fn gc_trace() {
    let mut driver = TestDriver::new(
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
    );

    let runtime = driver.runtime();
    let runtime_ref = runtime.borrow();

    let value: StructRef = invoke_fn!(runtime_ref, "new_foo").unwrap();
    let value = value.root(driver.runtime());

    assert_eq!(runtime_ref.gc_collect(), false);
    assert!(runtime_ref.gc_stats().allocated_memory > 0);

    drop(value);

    assert_eq!(runtime_ref.gc_collect(), true);
    assert_eq!(runtime_ref.gc_stats().allocated_memory, 0);
}

#[test]
fn map_struct_insert_field1() {
    let mut driver = TestDriver::new(
        r#"
        struct Foo {
            b: i64,
            c: f64,
        }

        pub fn foo_new(b: i64, c: f64) -> Foo {
            Foo { b, c }
        }
    "#,
    );

    let runtime = driver.runtime();
    let runtime_ref = runtime.borrow();

    let b = 5i64;
    let c = 3.0f64;
    let foo: StructRef = invoke_fn!(runtime_ref, "foo_new", b, c).unwrap();
    let foo = foo.root(driver.runtime());

    driver.update(
        runtime_ref,
        r#"
        struct Foo {
            a: i64,
            b: i64,
            c: f64,
        }
    "#,
    );
    assert_eq!(foo.by_ref().get::<i64>("a").unwrap(), 0);
    assert_eq!(foo.by_ref().get::<i64>("b").unwrap(), b);
    assert_eq!(foo.by_ref().get::<f64>("c").unwrap(), c);
}

#[test]
fn map_struct_insert_field2() {
    let mut driver = TestDriver::new(
        r#"
        struct Foo {
            a: i64,
            c: f64,
        }

        pub fn foo_new(a: i64, c: f64) -> Foo {
            Foo { a, c }
        }
    "#,
    );

    let runtime = driver.runtime();
    let runtime_ref = runtime.borrow();

    let a = 5i64;
    let c = 3.0f64;
    let foo: StructRef = invoke_fn!(runtime_ref, "foo_new", a, c).unwrap();
    let foo = foo.root(driver.runtime());

    driver.update(
        runtime_ref,
        r#"
        struct Foo {
            a: i64,
            b: f64,
            c: f64,
        }
    "#,
    );
    assert_eq!(foo.by_ref().get::<i64>("a").unwrap(), a);
    assert_eq!(foo.by_ref().get::<f64>("b").unwrap(), 0.0);
    assert_eq!(foo.by_ref().get::<f64>("c").unwrap(), c);
}

#[test]
fn map_struct_insert_field3() {
    let mut driver = TestDriver::new(
        r#"
        struct Foo {
            a: i64,
            b: f64,
        }

        pub fn foo_new(a: i64, b: f64) -> Foo {
            Foo { a, b }
        }
    "#,
    );

    let runtime = driver.runtime();
    let runtime_ref = runtime.borrow();

    let a = 5i64;
    let b = 3.0f64;
    let foo: StructRef = invoke_fn!(runtime_ref, "foo_new", a, b).unwrap();
    let foo = foo.root(driver.runtime());

    driver.update(
        runtime_ref,
        r#"
        struct Foo {
            a: i64,
            b: f64,
            c: f64,
        }
    "#,
    );
    assert_eq!(foo.by_ref().get::<i64>("a").unwrap(), a);
    assert_eq!(foo.by_ref().get::<f64>("b").unwrap(), b);
    assert_eq!(foo.by_ref().get::<f64>("c").unwrap(), 0.0);
}

#[test]
fn map_struct_remove_field1() {
    let mut driver = TestDriver::new(
        r#"
        struct Foo {
            a: f64,
            b: f64,
            c: i64,
        }

        pub fn foo_new(a: f64, b: f64, c: i64) -> Foo {
            Foo { a, b, c }
        }
    "#,
    );

    let runtime = driver.runtime();
    let runtime_ref = runtime.borrow();

    let a = 1.0f64;
    let b = 3.0f64;
    let c = 5i64;
    let foo: StructRef = invoke_fn!(runtime_ref, "foo_new", a, b, c).unwrap();
    let foo = foo.root(driver.runtime());

    driver.update(
        runtime_ref,
        r#"
        struct Foo {
            c: i64,
        }
    "#,
    );
    assert_eq!(foo.by_ref().get::<i64>("c").unwrap(), c);
}

#[test]
fn map_struct_remove_field2() {
    let mut driver = TestDriver::new(
        r#"
        struct Foo {
            a: f64,
            b: i64,
            c: f64,
        }

        pub fn foo_new(a: f64, b: i64, c: f64) -> Foo {
            Foo { a, b, c }
        }
    "#,
    );

    let runtime = driver.runtime();
    let runtime_ref = runtime.borrow();

    let a = 1.0f64;
    let b = 5i64;
    let c = 3.0f64;
    let foo: StructRef = invoke_fn!(runtime_ref, "foo_new", a, b, c).unwrap();
    let foo = foo.root(driver.runtime());

    driver.update(
        runtime_ref,
        r#"
        struct Foo {
            b: i64,
        }
    "#,
    );
    assert_eq!(foo.by_ref().get::<i64>("b").unwrap(), b);
}

#[test]
fn map_struct_remove_field3() {
    let mut driver = TestDriver::new(
        r#"
        struct Foo {
            a: i64,
            b: f64,
            c: f64,
        }

        pub fn foo_new(a: i64, b: f64, c: f64) -> Foo {
            Foo { a, b, c }
        }
    "#,
    );

    let runtime = driver.runtime();
    let runtime_ref = runtime.borrow();

    let a = 5i64;
    let b = 1.0f64;
    let c = 3.0f64;
    let foo: StructRef = invoke_fn!(runtime_ref, "foo_new", a, b, c).unwrap();
    let foo = foo.root(driver.runtime());

    driver.update(
        runtime_ref,
        r#"
        struct Foo {
            a: i64,
        }
    "#,
    );
    assert_eq!(foo.by_ref().get::<i64>("a").unwrap(), a);
}

#[test]
fn map_struct_cast_fields1() {
    let mut driver = TestDriver::new(
        r#"
        struct Foo(
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
    );

    let runtime = driver.runtime();
    let runtime_ref = runtime.borrow();

    let a = 1u8;
    let b = -2i16;
    let c = 3u32;
    let d = -4i64;
    let e = 3.14f32;
    let foo: StructRef = invoke_fn!(runtime_ref, "foo_new", a, b, c, d, e).unwrap();
    let foo = foo.root(driver.runtime());

    driver.update(
        runtime_ref,
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
    assert_eq!(foo.by_ref().get::<u16>("0").unwrap(), a.into());
    assert_eq!(foo.by_ref().get::<i32>("1").unwrap(), b.into());
    assert_eq!(foo.by_ref().get::<u64>("2").unwrap(), c.into());
    assert_eq!(foo.by_ref().get::<i128>("3").unwrap(), d.into());
    assert_eq!(foo.by_ref().get::<f64>("4").unwrap(), e.into());
}

#[test]
fn map_struct_cast_fields2() {
    let mut driver = TestDriver::new(
        r#"
        struct Foo(
            i16,
        )

        pub fn foo_new(a: i16) -> Foo {
            Foo(a)
        }
    "#,
    );

    let runtime = driver.runtime();
    let runtime_ref = runtime.borrow();

    let a = -2i16;
    let foo: StructRef = invoke_fn!(runtime_ref, "foo_new", a).unwrap();
    let foo = foo.root(driver.runtime());

    driver.update(
        runtime_ref,
        r#"
        struct Foo(
            u16,    // Cannot convert from `i16` to `u16`
        )
    "#,
    );

    assert_eq!(foo.by_ref().get::<u16>("0").unwrap(), 0);
}

#[test]
fn map_struct_swap_fields1() {
    let mut driver = TestDriver::new(
        r#"
        struct Foo {
            a: f64,
            b: i64,
            c: f64,
        }

        pub fn foo_new(a: f64, b: i64, c: f64) -> Foo {
            Foo { a, b, c }
        }
    "#,
    );

    let runtime = driver.runtime();
    let runtime_ref = runtime.borrow();

    let a = 1.0f64;
    let b = 3i64;
    let c = 5.0f64;
    let foo: StructRef = invoke_fn!(runtime_ref, "foo_new", a, b, c).unwrap();
    let foo = foo.root(driver.runtime());

    driver.update(
        runtime_ref,
        r#"
        struct Foo {
            c: f64,
            a: f64,
            b: i64,
        }
    "#,
    );
    assert_eq!(foo.by_ref().get::<f64>("a").unwrap(), a);
    assert_eq!(foo.by_ref().get::<i64>("b").unwrap(), b);
    assert_eq!(foo.by_ref().get::<f64>("c").unwrap(), c);
}

#[test]
fn map_struct_swap_fields2() {
    let mut driver = TestDriver::new(
        r#"
        struct Foo {
            a: f64,
            b: i64,
            c: f64,
            d: i64,
        }

        pub fn foo_new(a: f64, b: i64, c: f64, d: i64) -> Foo {
            Foo { a, b, c, d }
        }
    "#,
    );

    let runtime = driver.runtime();
    let runtime_ref = runtime.borrow();

    let a = 1.0f64;
    let b = 3i64;
    let c = 5.0f64;
    let d = 7i64;
    let foo: StructRef = invoke_fn!(runtime_ref, "foo_new", a, b, c, d).unwrap();
    let foo = foo.root(driver.runtime());

    driver.update(
        runtime_ref,
        r#"
        struct Foo {
            d: i64,
            c: f64,
            b: i64,
            a: f64,
        }
    "#,
    );
    assert_eq!(foo.by_ref().get::<f64>("a").unwrap(), a);
    assert_eq!(foo.by_ref().get::<i64>("b").unwrap(), b);
    assert_eq!(foo.by_ref().get::<f64>("c").unwrap(), c);
    assert_eq!(foo.by_ref().get::<i64>("d").unwrap(), d);
}

#[test]
fn map_struct_rename_field1() {
    let mut driver = TestDriver::new(
        r#"
        struct Foo {
            a: i64,
            b: f64,
            c: f64,
        }

        pub fn foo_new(a: i64, b: f64, c: f64) -> Foo {
            Foo { a, b, c }
        }
    "#,
    );

    let runtime = driver.runtime();
    let runtime_ref = runtime.borrow();

    let a = 5i64;
    let b = 1.0f64;
    let c = 3.0f64;
    let foo: StructRef = invoke_fn!(runtime_ref, "foo_new", a, b, c).unwrap();
    let foo = foo.root(driver.runtime());

    driver.update(
        runtime_ref,
        r#"
        struct Foo {
            a: i64,
            d: f64,
            c: f64,
        }
    "#,
    );
    assert_eq!(foo.by_ref().get::<i64>("a").unwrap(), a);
    assert_eq!(foo.by_ref().get::<f64>("d").unwrap(), b);
    assert_eq!(foo.by_ref().get::<f64>("c").unwrap(), c);
}

#[test]
fn map_struct_rename_field2() {
    let mut driver = TestDriver::new(
        r#"
        struct Foo {
            a: i64,
            b: f64,
            c: f64,
        }

        pub fn foo_new(a: i64, b: f64, c: f64) -> Foo {
            Foo { a, b, c }
        }
    "#,
    );

    let runtime = driver.runtime();
    let runtime_ref = runtime.borrow();

    let a = 5i64;
    let b = 1.0f64;
    let c = 3.0f64;
    let foo: StructRef = invoke_fn!(runtime_ref, "foo_new", a, b, c).unwrap();
    let foo = foo.root(driver.runtime());

    driver.update(
        runtime_ref,
        r#"
        struct Foo {
            d: i64,
            e: f64,
            f: f64,
        }
    "#,
    );
    assert_eq!(foo.by_ref().get::<i64>("d").unwrap(), a);
    assert_eq!(foo.by_ref().get::<f64>("e").unwrap(), b);
    assert_eq!(foo.by_ref().get::<f64>("f").unwrap(), c);
}

#[test]
fn map_struct_all() {
    let mut driver = TestDriver::new(
        r#"
        struct Foo {
            a: i32,
            b: f64,
            c: f64,
            d: i32,
        }

        pub fn foo_new(a: i32, b: f64, c: f64, d: i32) -> Foo {
            Foo { a, b, c, d }
        }
    "#,
    );

    let runtime = driver.runtime();
    let runtime_ref = runtime.borrow();

    let a = 5i32;
    let b = 1.0f64;
    let c = 3.0f64;
    let d = -1i32;
    let foo: StructRef = invoke_fn!(runtime_ref, "foo_new", a, b, c, d).unwrap();
    let foo = foo.root(driver.runtime());

    driver.update(
        runtime_ref,
        r#"
        struct Foo {
            b: f64, // move
        //  c: f64, // remove    
            d: i64, // move + convert
            e: i32, // move + rename
            f: i32, // add
        }
    "#,
    );
    assert_eq!(foo.by_ref().get::<f64>("b").unwrap(), b);
    assert_eq!(foo.by_ref().get::<i64>("d").unwrap(), d.into());
    assert_eq!(foo.by_ref().get::<i32>("e").unwrap(), a);
    assert_eq!(foo.by_ref().get::<i32>("f").unwrap(), 0);
}

#[test]
fn delete_used_struct() {
    let mut driver = TestDriver::new(
        r#"
        struct Foo {
            a: i64,
            b: f64,
            c: f64,
        }

        pub fn foo_new(a: i64, b: f64, c: f64) -> Foo {
            Foo { a, b, c }
        }
    "#,
    );

    let runtime = driver.runtime();
    let runtime_ref = runtime.borrow();

    let a = 5i64;
    let b = 1.0f64;
    let c = 3.0f64;
    let foo: StructRef = invoke_fn!(runtime_ref, "foo_new", a, b, c).unwrap();
    let foo = foo.root(driver.runtime());

    driver.update(
        runtime_ref,
        r#"
        struct Bar(i64);

        pub fn bar_new(a: i64) -> Bar {
            Bar(a)
        }
    "#,
    );

    assert!(driver
        .runtime()
        .borrow()
        .get_function_definition("foo_new")
        .is_none());
    assert!(driver
        .runtime()
        .borrow()
        .get_function_definition("bar_new")
        .is_some());
    assert_eq!(foo.by_ref().get::<i64>("a").unwrap(), a);
    assert_eq!(foo.by_ref().get::<f64>("b").unwrap(), b);
    assert_eq!(foo.by_ref().get::<f64>("c").unwrap(), c);
}

#[test]
fn nested_structs() {
    let mut driver = TestDriver::new(
        r#"
    struct(gc) GcStruct(f32, f32);
    struct(value) ValueStruct(f32, f32);

    struct(gc) GcWrapper(GcStruct, ValueStruct)
    struct(value) ValueWrapper(GcStruct, ValueStruct);

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
    );

    let runtime = driver.runtime();
    let runtime_ref = runtime.borrow();

    let a = -3.14f32;
    let b = 6.18f32;
    let gc_struct: StructRef = invoke_fn!(runtime_ref, "new_gc_struct", a, b).unwrap();
    let value_struct: StructRef = invoke_fn!(runtime_ref, "new_value_struct", a, b).unwrap();

    let gc_wrapper: StructRef = invoke_fn!(
        runtime_ref,
        "new_gc_wrapper",
        gc_struct.clone(),
        value_struct.clone()
    )
    .unwrap();

    let value_wrapper: StructRef = invoke_fn!(
        runtime_ref,
        "new_value_wrapper",
        gc_struct.clone(),
        value_struct.clone()
    )
    .unwrap();

    let gc_wrapper = gc_wrapper.root(driver.runtime());
    let value_wrapper = value_wrapper.root(driver.runtime());

    // Tests mapping of `gc -> gc`, `value -> value`
    driver.update(
        runtime_ref,
        r#"
    struct(gc) GcStruct(f64, f64);
    struct(value) ValueStruct(f64, f64);

    struct(gc) GcWrapper(GcStruct, ValueStruct)
    struct(value) ValueWrapper(GcStruct, ValueStruct);
    "#,
    );

    let gc_0 = gc_wrapper.by_ref().get::<StructRef>("0").unwrap();
    assert_eq!(gc_0.get::<f64>("0"), Ok(a.into()));
    assert_eq!(gc_0.get::<f64>("1"), Ok(b.into()));

    let gc_1 = gc_wrapper.by_ref().get::<StructRef>("1").unwrap();
    assert_eq!(gc_1.get::<f64>("0"), Ok(a.into()));
    assert_eq!(gc_1.get::<f64>("1"), Ok(b.into()));

    let value_0 = value_wrapper.by_ref().get::<StructRef>("0").unwrap();
    assert_eq!(value_0.get::<f64>("0"), Ok(a.into()));
    assert_eq!(value_0.get::<f64>("1"), Ok(b.into()));

    let value_1 = value_wrapper.by_ref().get::<StructRef>("1").unwrap();
    assert_eq!(value_1.get::<f64>("0"), Ok(a.into()));
    assert_eq!(value_1.get::<f64>("1"), Ok(b.into()));

    // Tests an identity mapping
    driver.update(
        runtime.borrow(),
        r#"
    struct(gc) GcStruct(f64, f64);
    struct(value) ValueStruct(f64, f64);

    struct(gc) GcWrapper(GcStruct, ValueStruct)
    struct(value) ValueWrapper(GcStruct, ValueStruct);
    "#,
    );

    let gc_0 = gc_wrapper.by_ref().get::<StructRef>("0").unwrap();
    assert_eq!(gc_0.get::<f64>("0"), Ok(a.into()));
    assert_eq!(gc_0.get::<f64>("1"), Ok(b.into()));

    let gc_1 = gc_wrapper.by_ref().get::<StructRef>("1").unwrap();
    assert_eq!(gc_1.get::<f64>("0"), Ok(a.into()));
    assert_eq!(gc_1.get::<f64>("1"), Ok(b.into()));

    let value_0 = value_wrapper.by_ref().get::<StructRef>("0").unwrap();
    assert_eq!(value_0.get::<f64>("0"), Ok(a.into()));
    assert_eq!(value_0.get::<f64>("1"), Ok(b.into()));

    let value_1 = value_wrapper.by_ref().get::<StructRef>("1").unwrap();
    assert_eq!(value_1.get::<f64>("0"), Ok(a.into()));
    assert_eq!(value_1.get::<f64>("1"), Ok(b.into()));

    let gc_0 = gc_0.root(driver.runtime());
    let gc_1 = gc_1.root(driver.runtime());
    let value_0 = value_0.root(driver.runtime());
    let value_1 = value_1.root(driver.runtime());

    // Tests mapping of `gc -> value`, `value -> gc`
    driver.update(
        runtime.borrow(),
        r#"
    struct(value) GcStruct(f64, f64);
    struct(gc) ValueStruct(f64, f64);

    struct(gc) GcWrapper(GcStruct, ValueStruct)
    struct(value) ValueWrapper(GcStruct, ValueStruct);
    "#,
    );

    assert_eq!(gc_0.by_ref().get::<f64>("0"), Ok(a.into()));
    assert_eq!(gc_0.by_ref().get::<f64>("1"), Ok(b.into()));

    assert_eq!(gc_1.by_ref().get::<f64>("0"), Ok(a.into()));
    assert_eq!(gc_1.by_ref().get::<f64>("1"), Ok(b.into()));

    assert_eq!(value_0.by_ref().get::<f64>("0"), Ok(a.into()));
    assert_eq!(value_0.by_ref().get::<f64>("1"), Ok(b.into()));

    assert_eq!(value_1.by_ref().get::<f64>("0"), Ok(a.into()));
    assert_eq!(value_1.by_ref().get::<f64>("1"), Ok(b.into()));

    // Tests mapping of different struct type, when `gc -> value`, `value -> gc`, and
    // retention of an old library (due to removal of `GcStruct` and `ValueStruct`)
    driver.update(
        runtime.borrow(),
        r#"
    struct(gc) GcStruct2(f64);
    struct(value) ValueStruct2(f64);

    struct(gc) GcWrapper(GcStruct2, ValueStruct2)
    struct(value) ValueWrapper(GcStruct2, ValueStruct2);
    "#,
    );

    // Existing, rooted objects should remain untouched
    assert_eq!(gc_0.by_ref().get::<f64>("0"), Ok(a.into()));
    assert_eq!(gc_0.by_ref().get::<f64>("1"), Ok(b.into()));

    assert_eq!(gc_1.by_ref().get::<f64>("0"), Ok(a.into()));
    assert_eq!(gc_1.by_ref().get::<f64>("1"), Ok(b.into()));

    assert_eq!(value_0.by_ref().get::<f64>("0"), Ok(a.into()));
    assert_eq!(value_0.by_ref().get::<f64>("1"), Ok(b.into()));

    assert_eq!(value_1.by_ref().get::<f64>("0"), Ok(a.into()));
    assert_eq!(value_1.by_ref().get::<f64>("1"), Ok(b.into()));

    // The values in the wrappers should have been updated
    let mut gc_0 = gc_wrapper.by_ref().get::<StructRef>("0").unwrap();
    assert_eq!(gc_0.get::<f64>("0"), Ok(0.0));
    gc_0.set::<f64>("0", a.into()).unwrap();

    let mut gc_1 = gc_wrapper.by_ref().get::<StructRef>("1").unwrap();
    assert_eq!(gc_1.get::<f64>("0"), Ok(0.0));
    gc_1.set::<f64>("0", a.into()).unwrap();

    let mut value_0 = value_wrapper.by_ref().get::<StructRef>("0").unwrap();
    assert_eq!(value_0.get::<f64>("0"), Ok(0.0));
    value_0.set::<f64>("0", a.into()).unwrap();

    let mut value_1 = value_wrapper.by_ref().get::<StructRef>("1").unwrap();
    assert_eq!(value_1.get::<f64>("0"), Ok(0.0));
    value_1.set::<f64>("0", a.into()).unwrap();

    let gc_0 = gc_0.root(driver.runtime());
    let gc_1 = gc_1.root(driver.runtime());
    let value_0 = value_0.root(driver.runtime());
    let value_1 = value_1.root(driver.runtime());

    // Tests mapping of different struct type, when `gc -> gc`, `value -> value`
    driver.update(
        runtime.borrow(),
        r#"
    struct(gc) GcStruct(f64, f64);
    struct(value) ValueStruct(f64, f64);

    struct(gc) GcWrapper(GcStruct, ValueStruct)
    struct(value) ValueWrapper(GcStruct, ValueStruct);
    "#,
    );

    // Existing, rooted objects should remain untouched
    assert_eq!(gc_0.by_ref().get::<f64>("0"), Ok(a.into()));
    assert_eq!(gc_1.by_ref().get::<f64>("0"), Ok(a.into()));
    assert_eq!(value_0.by_ref().get::<f64>("0"), Ok(a.into()));
    assert_eq!(value_1.by_ref().get::<f64>("0"), Ok(a.into()));

    // The values in the wrappers should have been updated
    let gc_0 = gc_wrapper.by_ref().get::<StructRef>("0").unwrap();
    assert_eq!(gc_0.get::<f64>("0"), Ok(0.0));
    assert_eq!(gc_0.get::<f64>("1"), Ok(0.0));

    let gc_1 = gc_wrapper.by_ref().get::<StructRef>("1").unwrap();
    assert_eq!(gc_1.get::<f64>("0"), Ok(0.0));
    assert_eq!(gc_1.get::<f64>("1"), Ok(0.0));

    let value_0 = value_wrapper.by_ref().get::<StructRef>("0").unwrap();
    assert_eq!(value_0.get::<f64>("0"), Ok(0.0));
    assert_eq!(value_0.get::<f64>("1"), Ok(0.0));

    let value_1 = value_wrapper.by_ref().get::<StructRef>("1").unwrap();
    assert_eq!(value_1.get::<f64>("0"), Ok(0.0));
    assert_eq!(value_1.get::<f64>("1"), Ok(0.0));
}

#[test]
fn insert_struct() {
    let mut driver = TestDriver::new(
        r#"
        struct Foo {
            a: i64,
            c: f64,
        }

        pub fn foo_new(a: i64, c: f64) -> Foo {
            Foo { a, c }
        }
    "#,
    );

    let runtime = driver.runtime();
    let runtime_ref = runtime.borrow();

    let a = 5i64;
    let c = 3.0f64;
    let foo: StructRef = invoke_fn!(runtime_ref, "foo_new", a, c).unwrap();
    let foo = foo.root(driver.runtime());

    driver.update(
        runtime_ref,
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

    assert_eq!(foo.by_ref().get::<i64>("a").unwrap(), a);
    assert_eq!(foo.by_ref().get::<f64>("c").unwrap(), c);

    let b = foo.by_ref().get::<StructRef>("b").unwrap();
    assert_eq!(b.get::<i64>("0"), Ok(0));

    let d = foo.by_ref().get::<StructRef>("d").unwrap();
    assert_eq!(d.get::<f64>("0"), Ok(0.0));
}

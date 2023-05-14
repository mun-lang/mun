#[macro_use]
mod util;

use mun_runtime::StructRef;
use mun_test::CompileAndRunTestDriver;

#[test]
fn reloadable_function_single_file() {
    let mut driver = CompileAndRunTestDriver::new(
        r"
    pub fn main() -> i32 { 5 }
    ",
        |builder| builder,
    )
    .expect("Failed to build test driver");
    assert_invoke_eq!(i32, 5, driver, "main");

    driver.update_file(
        "mod.mun",
        r"
    pub fn main() -> i32 { 10 }
    ",
    );
    assert_invoke_eq!(i32, 10, driver, "main");
}

#[test]
fn reloadable_function_multi_file() {
    let mut driver = CompileAndRunTestDriver::from_fixture(
        r#"
    //- /mun.toml
    [package]
    name="foo"
    version="0.0.0"

    //- /src/mod.mun
    use package::foo::bar;
    pub fn main() -> i32 { bar() }

    //- /src/foo.mun
    pub fn bar() -> i32 { 5 }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");
    assert_invoke_eq!(i32, 5, driver, "main");

    driver.update_file(
        "foo.mun",
        r#"
    pub fn bar() -> i32 { 10 }
    "#,
    );
    assert_invoke_eq!(i32, 10, driver, "main");
}

#[test]
fn reloadable_struct_decl_single_file() {
    let mut driver = CompileAndRunTestDriver::new(
        r#"
    pub struct(gc) Args {
        n: i32,
        foo: Bar,
    }
    
    struct(gc) Bar {
        m: i32,
    }

    pub fn args() -> Args {
        Args { n: 3, foo: Bar { m: 1 }, }
    }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let args: StructRef = driver
        .runtime
        .invoke("args", ())
        .expect("Failed to call function");

    let foo: StructRef = args.get("foo").expect("Failed to get struct field");
    assert_eq!(foo.get::<i32>("m").expect("Failed to get struct field"), 1);

    let foo = foo.root();

    driver.update_file(
        "mod.mun",
        r#"
    pub struct(gc) Args {
        n: i32,
        foo: Bar,
    }
    
    struct(gc) Bar {
        m: i64,
    }

    pub fn args() -> Args {
        Args { n: 3, foo: Bar { m: 1 }, }
    }
    "#,
    );

    let foo = foo.as_ref(&driver.runtime);
    assert_eq!(foo.get::<i64>("m").expect("Failed to get struct field"), 1);
}

#[test]
fn reloadable_struct_decl_multi_file() {
    let mut driver = CompileAndRunTestDriver::from_fixture(
        r#"
    //- /mun.toml
    [package]
    name="foo"
    version="0.0.0"

    //- /src/mod.mun
    use package::foo::Bar;
    pub struct(gc) Args {
        n: i32,
        foo: Bar,
    }

    pub fn args() -> Args {
        Args { n: 3, foo: Bar { m: 1 }, }
    }

    //- /src/foo.mun
    struct(gc) Bar {
        m: i64,
    }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let args: StructRef = driver
        .runtime
        .invoke("args", ())
        .expect("Failed to call function");

    assert_eq!(args.get::<i32>("n").expect("Failed to get struct field"), 3);

    let foo: StructRef = args.get("foo").expect("Failed to get struct field");
    assert_eq!(foo.get::<i64>("m").expect("Failed to get struct field"), 1);

    let args = args.root();
    let foo = foo.root();

    driver.update_file(
        "mod.mun",
        r#"
    use package::foo::Bar;
    pub struct(gc) Args {
        n: i64,
        foo: Bar,
    }

    pub fn args() -> Args {
        Args { n: 3, foo: Bar { m: 1 }, }
    }
    "#,
    );

    let args = args.as_ref(&driver.runtime);
    assert_eq!(args.get::<i64>("n").expect("Failed to get struct field"), 3);

    let foo = foo.as_ref(&driver.runtime);
    assert_eq!(foo.get::<i64>("m").expect("Failed to get struct field"), 1);
}

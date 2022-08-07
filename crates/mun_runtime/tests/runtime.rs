use mun_test::CompileAndRunTestDriver;
use std::io;

#[macro_use]
mod util;

#[test]
fn invoke() {
    let driver = CompileAndRunTestDriver::new(
        r#"
    pub fn sum(a: i32, b: i32) -> i32 { a + b }
        "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let result: i32 = driver.runtime.invoke("sum", (123i32, 456i32)).unwrap();
    assert_eq!(123 + 456, result);
}

#[test]
fn arrays_are_collected() {
    let driver = CompileAndRunTestDriver::new(
        r#"
    pub fn main() {
        let a = [1,2,3,4,5,6,7,8,9,1,2,3,4,5,6,7,8,9,1,2,3,4,5,6,7,8,9,1,2,3,4,5,6,7,8,9,]
    }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    assert_eq!(driver.runtime.gc_collect(), false);
    let _: () = driver
        .runtime
        .invoke("main", ())
        .expect("error invoking main function");
    assert_eq!(driver.runtime.gc_collect(), true);
    assert_eq!(driver.runtime.gc_collect(), false);
}

#[test]
fn arrays() {
    let driver = CompileAndRunTestDriver::new(
        r#"
    /// A version struct of explicitly 24 bits, this requires alignment.
    struct Version {
        major: u8,
        minor: u16,
    }

    /// Constructor function for a Version
    fn version(major: u8, minor: u16) -> Version {
        Version { major: major, minor: minor }
    }

    /// Returns true if a is considered smaller than b
    fn version_greater(a: Version, b: Version) -> bool {
        a.major > b.major || a.major == b.major && a.minor > b.minor
    }

    /// Performs bubble sort on an array of versions
    fn bubble_sort(array: [Version], len: usize) {
        let i = 0;
        while i<len {
            let j = 1;
            while j<len-i {
                if version_greater(array[j-1], array[j]) {
                    let tmp = array[j];
                    array[j] = array[j-1];
                    array[j-1] = tmp;
                }
                j += 1;
            }
            i += 1;
        }
    }

    pub fn main() -> u16 {
        let a = [version(3,4), version(1,2), version(4,5), version(0, 9), version(1,1)]
        bubble_sort(a, 5)
        a[0].minor
    }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    assert_invoke_eq!(u16, 9, driver, "main");
}

#[test]
fn multiple_modules() {
    let driver = CompileAndRunTestDriver::from_fixture(
        r#"
    //- /mun.toml
    [package]
    name="foo"
    version="0.0.0"

    //- /src/mod.mun
    pub fn main() -> i32 { foo::foo() }

    //- /src/foo.mun
    pub fn foo() -> i32 { 5 }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    assert_invoke_eq!(i32, 5, driver, "main");
}

#[test]
fn cyclic_modules() {
    let driver = CompileAndRunTestDriver::from_fixture(
        r#"
    //- /mun.toml
    [package]
    name="foo"
    version="0.0.0"

    //- /src/mod.mun
    pub fn main() -> i32 { foo::foo() }

    fn bar() -> i32 { 5 }

    //- /src/foo.mun
    pub fn foo() -> i32 { super::bar() }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");

    assert_invoke_eq!(i32, 5, driver, "main");
}

#[test]
fn from_fixture() {
    let driver = CompileAndRunTestDriver::from_fixture(
        r#"
    //- /mun.toml
    [package]
    name="foo"
    version="0.0.0"

    //- /src/mod.mun
    pub fn main() -> i32 { 5 }
    "#,
        |builder| builder,
    )
    .expect("Failed to build test driver");
    assert_invoke_eq!(i32, 5, driver, "main");
}

#[test]
fn error_assembly_not_linkable() {
    let driver = CompileAndRunTestDriver::new(
        r"
    extern fn dependency() -> i32;
    
    pub fn main() -> i32 { dependency() }
    ",
        |builder| builder,
    );
    assert_eq!(
        format!("{}", driver.unwrap_err()),
        format!(
            "{}",
            io::Error::new(
                io::ErrorKind::NotFound,
                "Failed to link due to missing dependencies.\n- dependency".to_string(),
            )
        )
    );
}

#[test]
fn arg_missing_bug() {
    let driver = CompileAndRunTestDriver::new(
        r"
    pub fn fibonacci_n() -> i64 {
        let n = arg();
        fibonacci(n)
    }

    fn arg() -> i64 {
        5
    }

    fn fibonacci(n: i64) -> i64 {
        if n <= 1 {
            n
        } else {
            fibonacci(n - 1) + fibonacci(n - 2)
        }
    }",
        |builder| builder,
    );
    driver.unwrap();
}

#[test]
fn cyclic_struct() {
    let driver = CompileAndRunTestDriver::new(
        r"
        pub struct Foo {
            foo: Foo
        }

        pub struct FooBar {
            bar: BarFoo
        }

        pub struct BarFoo {
            foo: FooBar
        }
        ",
        |builder| builder,
    )
    .unwrap();

    let foo_ty = driver.runtime.get_type_info_by_name("Foo").unwrap();
    let foo_foo_ty = foo_ty
        .as_struct()
        .unwrap()
        .fields()
        .find_by_name("foo")
        .unwrap()
        .ty();
    assert_eq!(foo_foo_ty, foo_ty);
}

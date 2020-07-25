#[macro_use]
mod util;

use util::*;

#[test]
fn hotreloadable() {
    let mut driver = TestDriver::new(
        r"
    pub fn main() -> i32 { 5 }
    ",
    );
    assert_invoke_eq!(i32, 5, driver, "main");

    let runtime = driver.runtime();
    driver.update(
        runtime.borrow(),
        r"
    pub fn main() -> i32 { 10 }
    ",
    );
    assert_invoke_eq!(i32, 10, driver, "main");
}

#[test]
fn hotreload_struct_decl() {
    let mut driver = TestDriver::new(
        r#"
    struct(gc) Args {
        n: i32,
        foo: Bar,
    }
    
    struct(gc) Bar {
        m: f64,
    }

    pub fn args() -> Args {
        Args { n: 3, foo: Bar { m: 1.0 }, }
    }
    "#,
    );

    let runtime = driver.runtime();
    driver.update(
        runtime.borrow(),
        r#"
    struct(gc) Args {
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
    );
}

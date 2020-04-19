#[macro_use]
mod util;

use util::*;

#[test]
fn hotreloadable() {
    let mut driver = TestDriver::new(
        r"
    pub fn main() -> int { 5 }
    ",
    );
    assert_invoke_eq!(i64, 5, driver, "main");
    driver.update(
        r"
    pub fn main() -> int { 10 }
    ",
    );
    assert_invoke_eq!(i64, 10, driver, "main");
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

    pub fn args() -> Args {
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

    pub fn args() -> Args {
        Args { n: 3, foo: Bar { m: 1 }, }
    }
    "#,
    );
}

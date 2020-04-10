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

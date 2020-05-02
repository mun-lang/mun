mod util;

use std::io;
use util::*;

#[test]
fn error_assembly_not_linkable() {
    let mut driver = TestDriver::new(
        r"
    extern fn dependency() -> i32;
    
    pub fn main() -> i32 { dependency() }
    ",
    );

    assert_eq!(
        format!("{}", driver.spawn().unwrap_err()),
        format!(
            "{}",
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("Failed to link: function `dependency` is missing."),
            )
        )
    );
}

#[test]
fn arg_missing_bug() {
    let mut driver = TestDriver::new(
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
    );

    driver.spawn().unwrap()
}

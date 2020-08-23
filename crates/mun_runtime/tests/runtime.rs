use mun_test::CompileAndRunTestDriver;
use std::io;

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
                format!("Failed to link: function `dependency` is missing."),
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

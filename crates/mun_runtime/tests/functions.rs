#[macro_use]
mod util;

use mun_test::CompileAndRunTestDriver;

#[test]
fn unknown_function() {
    let driver = CompileAndRunTestDriver::new(
        r"
    pub fn main() -> i32 { 5 }
    ",
        |builder| builder,
    )
    .expect("Failed to build test driver");

    const EXPECTED_FN_NAME: &str = "may";

    let result: Result<i32, _> = driver.runtime.invoke(EXPECTED_FN_NAME, ());
    let err = result.unwrap_err();

    assert_eq!(
        err.to_string(),
        format!(
            "failed to obtain function '{}', no such function exists.",
            EXPECTED_FN_NAME
        )
    );
}

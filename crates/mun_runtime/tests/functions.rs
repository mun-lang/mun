#[macro_use]
mod util;

use mun_test::CompileAndRunTestDriver;

#[test]
fn unknown_function() {
    const EXPECTED_FN_NAME: &str = "may";

    let driver = CompileAndRunTestDriver::new(
        r"
    pub fn main() -> i32 { 5 }
    ",
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let result: Result<i32, _> = driver.runtime.invoke(EXPECTED_FN_NAME, ());
    let err = result.unwrap_err();

    assert_eq!(
        err.to_string(),
        format!("failed to obtain function '{EXPECTED_FN_NAME}', no such function exists.")
    );
}

#[test]
fn exact_case_sensitive_match_exists_function() {
    const EXPECTED_FN_NAME: &str = "Foo";

    let driver = CompileAndRunTestDriver::new(
        r"
    pub fn main() -> i32 { 5 }
    pub fn foo() -> i32 { 4 }
    pub fn bar() -> i32 { 3 }
    ",
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let result: Result<i32, _> = driver.runtime.invoke(EXPECTED_FN_NAME, ());
    let err = result.unwrap_err();

    assert_eq!(
        err.to_string(),
        format!(
            "failed to obtain function '{}', no such function exists. There is a function with a similar name: {}",
            EXPECTED_FN_NAME, EXPECTED_FN_NAME.to_lowercase()
        )
    );
}

#[test]
fn close_match_exists_function() {
    const EXPECTED_FN_NAME: &str = "calculatedistance";

    let driver = CompileAndRunTestDriver::new(
        r"
    pub fn main() -> i32 { 5 }
    pub fn calculate_distance() -> i32 { 4 }
    pub fn bar() -> i32 { 3 }
    ",
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let result: Result<i32, _> = driver.runtime.invoke(EXPECTED_FN_NAME, ());
    let err = result.unwrap_err();

    assert_eq!(
        err.to_string(),
        format!(
            "failed to obtain function '{EXPECTED_FN_NAME}', no such function exists. There is a function with a similar name: calculate_distance"
        )
    );
}

#[test]
fn no_close_match_exists_function() {
    const EXPECTED_FN_NAME: &str = "calculate";

    let driver = CompileAndRunTestDriver::new(
        r"
    pub fn main() -> i32 { 5 }
    pub fn calculate_distance() -> i32 { 4 }
    ",
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let result: Result<i32, _> = driver.runtime.invoke(EXPECTED_FN_NAME, ());
    let err = result.unwrap_err();

    assert_eq!(
        err.to_string(),
        format!("failed to obtain function '{EXPECTED_FN_NAME}', no such function exists.")
    );
}

#[test]
fn multiple_match_exists_function() {
    const EXPECTED_FN_NAME: &str = "foobar";

    let driver = CompileAndRunTestDriver::new(
        r"
    pub fn main() -> i32 { 5 }
    pub fn foobar_a() -> i32 { 4 }
    pub fn foobar_b() -> i32 { 4 }
    ",
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let result: Result<i32, _> = driver.runtime.invoke(EXPECTED_FN_NAME, ());
    let err = result.unwrap_err();

    assert_eq!(
        err.to_string(),
        format!(
            "failed to obtain function '{EXPECTED_FN_NAME}', no such function exists. There is a function with a similar name: foobar_b"
        )
    );
}

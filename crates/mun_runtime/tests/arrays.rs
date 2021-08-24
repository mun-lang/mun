use mun_runtime::{ArrayRef, StructRef};
use mun_test::CompileAndRunTestDriver;

#[test]
fn arrays() {
    let driver = CompileAndRunTestDriver::new(
        r"
    pub fn main() -> [i32] { [5,4,3,2,1] }
    ",
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let result: ArrayRef<'_, i32> = driver.runtime.invoke("main", ()).unwrap();

    assert_eq!(result.len(), 5);
    assert!(result.capacity() >= 5);
    assert_eq!(result.iter().collect::<Vec<_>>(), vec![5, 4, 3, 2, 1]);
}

#[test]
fn array_of_structs() {
    let driver = CompileAndRunTestDriver::new(
        r"
    pub struct Number { value: i32 };

    pub fn main() -> [Number] { [Number { value: 2351 }, Number { value: 18571 }] }
    ",
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let result: ArrayRef<'_, StructRef> = driver.runtime.invoke("main", ()).unwrap();
    let number: i32 = result.iter().nth(1).unwrap().get("value").unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(number, 18571);
}

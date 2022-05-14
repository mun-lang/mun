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

#[test]
fn arrays_as_argument() {
    let driver = CompileAndRunTestDriver::new(
        r"
    pub fn generate() -> [i32] { [5,4,3,2,1] }
    pub fn add_one(array: [i32], len: usize) -> [i32] {
        let i = 0;
        loop {
            array[i] += 1;
            i += 1;
            if i >= len {
                break array
            }
        }
    }
    ",
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let result: ArrayRef<'_, i32> = driver.runtime.invoke("generate", ()).unwrap();

    assert_eq!(result.len(), 5);
    assert!(result.capacity() >= 5);
    assert_eq!(result.iter().collect::<Vec<_>>(), vec![5, 4, 3, 2, 1]);

    let result_array: ArrayRef<'_, i32> = driver
        .runtime
        .invoke("add_one", (result.clone(), result.len()))
        .unwrap();

    assert_eq!(result_array.len(), 5);
    assert!(result_array.capacity() >= 5);
    assert_eq!(result_array.iter().collect::<Vec<_>>(), vec![6, 5, 4, 3, 2]);
}

#[test]
fn root_array() {
    let driver = CompileAndRunTestDriver::new(
        r"
    pub fn main() -> [i32] { [5,4,3,2,1] }
    ",
        |builder| builder,
    )
    .expect("Failed to build test driver");

    let result = {
        let array: ArrayRef<i32> = driver.runtime.invoke("main", ()).unwrap();
        array.root()
    };

    let result = result.as_ref(&driver.runtime);
    assert_eq!(result.len(), 5);
    assert!(result.capacity() >= 5);
    assert_eq!(result.iter().collect::<Vec<_>>(), vec![5, 4, 3, 2, 1]);
}

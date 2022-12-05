# extern crate mun_runtime;
use mun_runtime::{Runtime, ArrayRef};
use std::env;

fn main() {
    let lib_path = env::args().nth(1).expect("Expected path to a Mun library.");

    // Safety: We assume that the library that is loaded is a valid munlib
    let builder = Runtime::builder(lib_path);
    let mut runtime = unsafe { builder.finish() }
        .expect("Failed to spawn Runtime");

    let input = ArrayRef<'_, u64> = runtime.invoke("generate", ()).unwrap();

    assert_eq!(input.len(), 5);
    assert!(input.capacity() >= 5);

    let output: ArrayRef<'_, u64> = runtime
        .invoke("add_one", (input.clone(), input.len()))
        .unwrap();

    assert_eq!(output.iter().collect::<Vec<_>>(), vec![6, 5, 4, 3, 2]);
}

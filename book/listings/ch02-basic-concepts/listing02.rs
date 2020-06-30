use mun_runtime::{invoke_fn, RuntimeBuilder};
use std::{cell::RefCell, rc::Rc};

fn main() {
    let runtime = RuntimeBuilder::new("main.munlib")
        .spawn()
        .expect("Failed to spawn Runtime");

    let runtime_ref = runtime.borrow();
    let result: bool = invoke_fn!(runtime_ref, "random_bool").unwrap();
    println!("random bool: {}", result);
}

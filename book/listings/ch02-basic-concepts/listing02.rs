use mun_runtime::{invoke_fn, RetryResultExt, RuntimeBuilder};
use std::{cell::RefCell, rc::Rc};

fn main() {
    let runtime = RuntimeBuilder::new("main.munlib")
        .spawn()
        .expect("Failed to spawn Runtime");

    let result: bool = invoke_fn!(runtime, "random_bool").unwrap();
    println!("random bool: {}", result);
}

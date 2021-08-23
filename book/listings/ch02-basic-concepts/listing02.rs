use mun_runtime::RuntimeBuilder;
use std::{cell::RefCell, rc::Rc};

fn main() {
    let runtime = RuntimeBuilder::new("main.munlib")
        .spawn()
        .expect("Failed to spawn Runtime");

    let runtime_ref = runtime.borrow();
    let result: bool = runtime_ref.invoke("random_bool", ()).unwrap();
    println!("random bool: {}", result);
}

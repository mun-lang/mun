use mun_runtime::Runtime;
use std::{cell::RefCell, rc::Rc};

fn main() {
    let runtime = Runtime::builder("main.munlib")
        .finish()
        .expect("Failed to spawn Runtime");

    let result: bool = runtime.invoke("random_bool", ()).unwrap();
    println!("random bool: {}", result);
}

use mun_runtime::{invoke_fn, RetryResultExt, RuntimeBuilder};
use std::{cell::RefCell, rc::Rc};

extern "C" fn random() -> i64 {
    let result = std::time::Instant::now().elapsed().subsec_nanos() as i64;
    println!("random: {}", result);
    result
}

fn main() {
    let runtime = RuntimeBuilder::new("main.munlib")
        .insert_fn("random", random as extern "C" fn() -> i64)
        .spawn()
        .expect("Failed to spawn Runtime");

    let result: bool = invoke_fn!(runtime, "random_bool").unwrap();
    println!("random_bool: {}", result);
}

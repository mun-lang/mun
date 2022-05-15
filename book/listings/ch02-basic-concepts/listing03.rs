use mun_runtime::Runtime;
use std::{cell::RefCell, rc::Rc};

extern "C" fn random() -> i64 {
    let result = std::time::Instant::now().elapsed().subsec_nanos() as i64;
    println!("random: {}", result);
    result
}

fn main() {
    let runtime = Runtime::builder("main.munlib")
        .insert_fn("random", random as extern "C" fn() -> i64)
        .finish()
        .expect("Failed to spawn Runtime");

    let result: bool = runtime.invoke("random_bool", ()).unwrap();
    println!("random_bool: {}", result);
}

use mun_runtime::{invoke_fn, RetryResultExt, RuntimeBuilder};
use std::{cell::RefCell, env, rc::Rc};

fn main() {
    let lib_path = env::args().nth(1).expect("Expected path to a Mun library.");

    let mut runtime = RuntimeBuilder::new(lib_path)
        .spawn()
        .expect("Failed to spawn Runtime");

    loop {
        let arg: i64 = invoke_fn!(runtime, "arg").wait();
        let result: i64 = invoke_fn!(runtime, "fibonacci").wait();
        println!("fibonacci({}) = {}", arg, result);
        runtime.borrow_mut().update();
    }
}

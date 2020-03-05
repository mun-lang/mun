use mun_runtime::{invoke_fn, RetryResultExt, RuntimeBuilder};
use std::cell::RefCell;
use std::env;
use std::rc::Rc;

// How to run?
// 1. On the CLI, navigate to the `crates/mun_runtime/examples` directory.
// 2. Run the compiler daemon from the CLI: `/path/to/mun build resources/fibonacci.mun --watch`
// 3. Run the application from the CLI: cargo run --example hot_reloading -- fibonacci.dll
fn main() {
    let lib_dir = env::args().nth(1).expect("Expected path to a Mun library.");
    println!("lib: {}", lib_dir);

    let runtime = RuntimeBuilder::new(lib_dir)
        .spawn()
        .expect("Failed to spawn Runtime");

    let runtime = Rc::new(RefCell::new(runtime));
    loop {
        let n: i64 = invoke_fn!(runtime, "nth").wait();
        let result: i64 = invoke_fn!(runtime, "fibonacci", n).wait();
        println!("fibonacci({}) = {}", n, result);
        runtime.borrow_mut().update();
    }
}

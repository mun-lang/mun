use mun_runtime::RuntimeBuilder;
use std::env;

// How to run?
// 1. On the CLI, navigate to the `crates/mun_runtime/examples` directory.
// 2. Run the compiler daemon from the CLI: `/path/to/mun build resources/fibonacci.mun --watch`
// 3. Run the application from the CLI: cargo run --example fibonacci -- fibonacci.munlib
fn main() {
    let lib_dir = env::args().nth(1).expect("Expected path to a Mun library.");
    println!("lib: {}", lib_dir);

    let runtime = RuntimeBuilder::new(lib_dir)
        .spawn()
        .expect("Failed to spawn Runtime");

    let mut runtime_ref = runtime.borrow_mut();

    loop {
        let n: i64 = runtime_ref
            .invoke("nth", ())
            .unwrap_or_else(|e| e.wait(&mut runtime_ref));
        let result: i64 = runtime_ref
            .invoke("fibonacci", (n,))
            .unwrap_or_else(|e| e.wait(&mut runtime_ref));
        println!("fibonacci({}) = {}", n, result);
        runtime_ref.update();
    }
}

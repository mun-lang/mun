use mun_runtime::Runtime;
use std::env;

// How to run?
// 1. On the CLI, navigate to the `crates/mun_runtime/examples` directory.
// 2. Run the compiler daemon from the CLI: `/path/to/mun build resources/fibonacci.mun --watch`
// 3. Run the application from the CLI: cargo run --example fibonacci -- fibonacci.munlib
fn main() {
    let lib_dir = env::args().nth(1).expect("Expected path to a Mun library.");
    println!("lib: {}", lib_dir);

    // Safety: we assume here that the library passed on the commandline is safe.
    let mut runtime =
        unsafe { Runtime::builder(lib_dir).finish() }.expect("Failed to spawn Runtime");

    loop {
        let n: i64 = runtime
            .invoke("nth", ())
            .unwrap_or_else(|e| e.wait(&mut runtime));
        let result: i64 = runtime
            .invoke("fibonacci", (n,))
            .unwrap_or_else(|e| e.wait(&mut runtime));
        println!("fibonacci({}) = {}", n, result);

        // Safety: we assume the updates are safe.
        unsafe { runtime.update() };
    }
}

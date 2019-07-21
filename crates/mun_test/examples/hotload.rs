extern crate mun_runtime;

use mun_runtime::{MunRuntime, Symbol};
use std::path::Path;
use std::thread;
use std::time::Duration;

fn main() {
    let mut runtime =
        MunRuntime::new(Duration::from_secs(1)).expect("Failed to initialize Mun runtime.");

    let input_path = Path::new("crates/mun_test/src/");
    let output_path = Path::new("target/debug/test.dll");

    runtime
        .add_module(&input_path, &output_path, false)
        .expect("Failed to load shared library.");

    loop {
        runtime.update();

        let add_fn: Symbol<unsafe extern "C" fn(f32, f32) -> f32> = runtime
            .get_symbol(&input_path, "add")
            .expect("Could not find 'add' function symbol.");

        println!("2.0 + 2.0 = {}", unsafe { add_fn(2.0, 2.0) });

        thread::sleep(Duration::from_secs(1));
    }
}

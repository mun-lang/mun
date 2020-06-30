use mun_runtime::{invoke_fn, RuntimeBuilder, StructRef};
use std::{env, time};

extern "C" fn log_f32(value: f32) {
    println!("{}", value);
}

// How to run?
// 1. On the CLI, navigate to the `crates/mun_runtime/examples` directory.
// 2. Run the compiler daemon from the CLI: `/path/to/mun build resources/buoyancy.mun --watch`
// 3. Run the application from the CLI: cargo run --example buoyancy -- buoyancy.munlib
fn main() {
    let lib_dir = env::args().nth(1).expect("Expected path to a Mun library.");

    let runtime = RuntimeBuilder::new(lib_dir)
        .insert_fn("log_f32", log_f32 as extern "C" fn(f32))
        .spawn()
        .expect("Failed to spawn Runtime");

    let runtime_ref = runtime.borrow();
    let ctx: StructRef = invoke_fn!(runtime_ref, "new_sim").unwrap();

    let mut previous = time::Instant::now();
    const FRAME_TIME: time::Duration = time::Duration::from_millis(40);
    loop {
        let now = time::Instant::now();
        let elapsed = now.duration_since(previous);

        let elapsed_secs = if elapsed < FRAME_TIME {
            std::thread::sleep(FRAME_TIME - elapsed);
            FRAME_TIME.as_secs_f32()
        } else {
            elapsed.as_secs_f32()
        };

        let runtime_ref = runtime.borrow();
        let _: () = invoke_fn!(runtime_ref, "sim_update", ctx.clone(), elapsed_secs).unwrap();
        previous = now;

        runtime.borrow_mut().update();
    }
}

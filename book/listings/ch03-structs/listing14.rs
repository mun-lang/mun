# extern crate mun_runtime;
use mun_runtime::{invoke_fn, RetryResultExt, RuntimeBuilder, StructRef};
use std::{env, time};

extern "C" fn log_f32(value: f32) {
    println!("{}", value);
}

fn main() {
    let lib_dir = env::args().nth(1).expect("Expected path to a Mun library.");

    let runtime = RuntimeBuilder::new(lib_dir)
        .insert_fn("log_f32", log_f32 as extern "C" fn(f32))
        .spawn()
        .expect("Failed to spawn Runtime");

    let ctx: StructRef = invoke_fn!(runtime, "new_sim").wait();

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

        let _: () = invoke_fn!(runtime, "sim_update", ctx.clone(), elapsed_secs).wait();
        previous = now;

        runtime.borrow_mut().update();
    }
}

# extern crate mun_runtime;
use mun_runtime::{Runtime, StructRef};
use std::{env, time};

extern "C" fn log_f32(value: f32) {
    println!("{}", value);
}

fn main() {
    let lib_dir = env::args().nth(1).expect("Expected path to a Mun library.");

    // Safety: We assume that the library that is loaded is a valid munlib
    let builder = Runtime::builder(lib_dir)
        .insert_fn("log_f32", log_f32 as extern "C" fn(f32));
    let mut runtime = unsafe { builder.finish() }
        .expect("Failed to spawn Runtime");

    let ctx = runtime.invoke::<StructRef, ()>("new_sim", ()).unwrap().root();

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

        let _: () = runtime.invoke("sim_update", (ctx.as_ref(&runtime), elapsed_secs)).unwrap();
        previous = now;

        unsafe { runtime.update() };
    }
}

use std::{env, path::PathBuf};

use bevy::prelude::*;
use mun_runtime::Runtime as MunRuntime;

// How to run?
// 1. On the CLI, navigate to the `examples/bevy_simple` directory.
// 2. Run the compiler daemon from the CLI: `/path/to/mun build --watch`
// 3. Run the application from the CLI: cargo run --example bevy_simple -- target/mod.munlib

// Minimal Bevy application that demonstrates how to insert the Mun runtime into a Bevy world object and
// utilizes the Mun runtime inside of Bevy systems.
fn main() {
    let lib_dir = PathBuf::from(env::args().nth(1).expect("Expected path to a Mun library."));

    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugin(MunPlugin { lib_dir })
        .run();
}

// A Bevy best-practice is to build your logical separations of code into "Plugins".
// This example will build the core Mun functionality into a MunPlugin. This plugin is loaded into
// Bevy in the "main()" function above. Through this plugin we will load and access the various
// Mun pieces with Bevy.
struct MunPlugin {
    lib_dir: PathBuf,
}

impl Plugin for MunPlugin {
    fn build(&self, app: &mut App) {
        // A "resource" is similar to a global variable of the Bevy "world" (main application).
        // Bevy handles most systems (functions) in a parallel / multi-threaded way by default.
        // Actions such as Initializing, Updating, or Calling functions using the Mun runtime
        // can not be performed in parallel with other Bevy systems. At least not without
        // additional overhead.
        // We are using "exclusive_system()" to load the initial setup that inserts the Mun
        // runtime as a Bevy world resource, as well as other resources.
        // "exclusive_system()" is only necessary for loading the Mun runtime.
        app.insert_resource(self.lib_dir.clone())
            .add_startup_system(setup.exclusive_system())
            .add_system(print_from_mun)
            .add_system(reload_munlib_every_frame);
    }
}

struct PrintTimer(Timer);

// Insert the Mun runtime, and associated timers as Bevy world resources.
fn setup(world: &mut World) {
    let builder = MunRuntime::builder(
        world
            .get_resource::<PathBuf>()
            .expect("Lib path must be added as resource"),
    );
    // We assume the Mun runtime is safe.
    let mun: MunRuntime = unsafe { builder.finish() }.expect("Failed to spawn Runtime");
    // Mun does not implement the send/sync trait so it needs to be inserted into Bevy as a
    // "non_send_resource".
    world.insert_non_send_resource(mun);
    world.insert_resource(PrintTimer(Timer::from_seconds(1.0, true)));

    world.remove_resource::<PathBuf>();
}

fn reload_munlib_every_frame(
    // Since Mun was loaded with "non_send_resource" it is accessed as "NonSend" / "NonSendMut"
    // instead of "Res" / "ResMut". "NonSend" is a data type that must be accessed from Bevy's main
    // thread. This is typically reserved for data that is not safe to access in a multi-threaded
    // environment.
    mut mun: NonSendMut<MunRuntime>,
) {
    let _ = unsafe { mun.update() };
}

fn print_from_mun(mun: NonSend<MunRuntime>, time: Res<Time>, mut timer: ResMut<PrintTimer>) {
    // Call the function defined in Mun named "mun_func"
    let result: usize = mun.invoke("mun_func", ()).unwrap();
    if timer.0.tick(time.delta()).just_finished() {
        println!("Printing value from `mun_func`: {}", result);
    }
}

use bevy::prelude::*;
use mun_runtime::Runtime as MunRuntime;

const MUNLIB_PATH: &str = "mun/target/mod.munlib";

// Minimal Bevy application.
fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_plugin(MunPlugin)
        .run();
}

// A Bevy best-practice is to build your logical separations of code into "Plugins".
// This example will build the core Mun functionality into a MunPlugin. This plugin is loaded into
// Bevy in the "main()" function above. Through this plugin we will load and access the various
// Mun pieces with Bevy.
struct MunPlugin;

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
        app.add_startup_system(setup.exclusive_system())
            .add_system(print_from_mun)
            .add_system(reload_munlib_every_frame);
    }
}

struct PrintTimer(Timer);

// Insert the Mun runtime, and associated timers as Bevy world resources.
fn setup(world: &mut World) {
    let builder = MunRuntime::builder(MUNLIB_PATH);
    // We assume the Mun runtime is safe.
    let mun: MunRuntime = unsafe { builder.finish() }.expect("Failed to spawn Runtime");
    // Mun does not implement the send/sync trait so it needs to be inserted into Bevy as a
    // "non_send_resource".
    world.insert_non_send_resource(mun);
    world.insert_resource(PrintTimer(Timer::from_seconds(1.0, true)));
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

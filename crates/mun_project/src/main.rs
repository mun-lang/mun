extern crate mun_runtime;
extern crate mun_symbols;

use mun_runtime::{MunRuntime, Symbol};
use mun_symbols::prelude::*;
use std::path::Path;
use std::thread;
use std::time::Duration;

fn main() {
    let mut runtime =
        MunRuntime::new(Duration::from_secs(1)).expect("Failed to initialize Mun runtime.");

    let manifest_path = Path::new("..\\mun_test\\Cargo.toml");

    runtime
        .add_manifest(&manifest_path)
        .expect("Failed to load shared library.");

    loop {
        runtime.update();

        let symbols_fn: Symbol<fn() -> &'static ModuleInfo> = runtime
            .get_symbol(&manifest_path, "symbols")
            .expect("Could not find 'symbols' function symbol.");

        println!("{:?}", symbols_fn());

        let add_fn: Symbol<fn(f32, f32) -> f32> = runtime
            .get_symbol(&manifest_path, "add")
            .expect("Could not find 'add' function symbol.");

        println!("2.0 + 2.0 = {}", add_fn(2.0, 2.0));

        thread::sleep(Duration::from_secs(1));
    }
}

#[cfg(test)]
mod tests {
    use mun_runtime::{MunRuntime, Symbol};
    use std::path::Path;
    use std::time::Duration;

    #[test]
    fn mun_fn_call() {
        let mut runtime =
            MunRuntime::new(Duration::from_secs(1)).expect("Failed to initialize Mun runtime.");

        let manifest_path = Path::new("..\\mun_test\\Cargo.toml");

        runtime
            .add_manifest(&manifest_path)
            .expect("Failed to load shared library.");

        let add_fn: Symbol<unsafe extern "C" fn(f32, f32) -> f32> = runtime
            .get_symbol(&manifest_path, "add")
            .expect("Could not find 'add' function symbol.");

        assert_eq!(unsafe { add_fn(2.0, 2.0) }, 4.0);
    }
}

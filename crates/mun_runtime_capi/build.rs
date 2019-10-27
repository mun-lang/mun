use std::env;

use cbindgen;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let bindings = cbindgen::generate(crate_dir).expect("Unable to generate Mun Runtime bindings.");
    bindings.write_to_file("c/include/mun/runtime.h");
    bindings.write_to_file("cpp/include/mun/runtime_capi.h");
}

use std::env;

use cbindgen;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    cbindgen::generate(crate_dir)
        .expect("Unable to generate Mun Runtime bindings.")
        .write_to_file("ffi/include/mun/runtime_capi.h");
}

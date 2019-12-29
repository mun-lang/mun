use crate::{project_root, update, Result};
use teraron::Mode;

pub const RUNTIME_CAPI_DIR: &str = "crates/mun_runtime_capi";

/// Generates the FFI bindings for the Mun runtime
pub fn generate(mode: Mode) -> Result<()> {
    let crate_dir = project_root().join(RUNTIME_CAPI_DIR);
    let file_path = crate_dir.join("ffi/include/mun/runtime_capi.h");

    let mut file_contents = Vec::<u8>::new();
    cbindgen::generate(crate_dir)?.write(&mut file_contents);

    let file_contents = String::from_utf8(file_contents)?;
    update(&file_path, &file_contents, mode)
}

use crate::{project_root, update, Result};
use teraron::Mode;

pub const ABI_DIR: &str = "crates/mun_abi";

/// Generates the FFI bindings for the Mun ABI
pub fn generate(mode: Mode) -> Result<()> {
    let crate_dir = project_root().join(ABI_DIR);
    let file_path = crate_dir.join("ffi/include/mun_abi.h");

    let mut file_contents = Vec::<u8>::new();
    cbindgen::generate(crate_dir)?.write(&mut file_contents);

    let file_contents = String::from_utf8(file_contents)?;
    update(&file_path, &file_contents, mode)
}

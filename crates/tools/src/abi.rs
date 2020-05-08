use crate::{project_root, reformat, update, Result};
use failure::format_err;
use teraron::Mode;

pub const ABI_DIR: &str = "crates/mun_abi";

use bindgen::{self, callbacks::EnumVariantValue, callbacks::ParseCallbacks};

#[derive(Debug)]
struct RemoveVendorName;

impl ParseCallbacks for RemoveVendorName {
    fn enum_variant_name(
        &self,
        enum_name: Option<&str>,
        original_variant_name: &str,
        _variant_value: EnumVariantValue,
    ) -> Option<String> {
        Some(
            original_variant_name
                .trim_start_matches(enum_name.unwrap_or(""))
                .to_string(),
        )
    }

    fn item_name(&self, original_item_name: &str) -> Option<String> {
        if original_item_name == "MunPrivacy_t" {
            Some("Privacy".to_string())
        } else if original_item_name == "MunTypeGroup_t" {
            Some("TypeGroup".to_string())
        } else if original_item_name == "MunStructMemoryKind_t" {
            Some("StructMemoryKind".to_string())
        } else {
            Some(original_item_name.trim_start_matches("Mun").to_string())
        }
    }
}

/// Generates the FFI bindings for the Mun ABI
pub fn generate(mode: Mode) -> Result<()> {
    let crate_dir = project_root().join(ABI_DIR);
    let output_file_path = crate_dir.join("src/autogen.rs");
    let input_file_path = crate_dir.join("c/include/mun_abi.h");

    let input_file_str = input_file_path
        .to_str()
        .ok_or_else(|| failure::err_msg("could not create path to mun_abi.h"))?;
    let bindings = bindgen::Builder::default()
        .header(input_file_str)
        .whitelist_type("Mun.*")
        // Remove type aliasing on Linux
        .blacklist_type("__uint8_t")
        .blacklist_type("__uint16_t")
        .blacklist_type("__uint32_t")
        .blacklist_type("__uint64_t")
        .parse_callbacks(Box::new(RemoveVendorName))
        // FIXME: Prevent double derivation of Copy and Debug attributes on Windows
        .derive_copy(false)
        .derive_debug(false)
        .raw_line("#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]")
        .raw_line("use crate::{StructMemoryKind, TypeGroup};")
        .generate()
        .map_err(|_| format_err!("Unable to generate bindings from 'mun_abi.h'"))?;

    let file_contents = reformat(bindings.to_string())?;
    update(&output_file_path, &file_contents, mode)
}

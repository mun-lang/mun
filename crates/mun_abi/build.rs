use std::env;
use std::path::PathBuf;

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
        } else {
            Some(original_item_name.trim_start_matches("Mun").to_string())
        }
    }
}

fn main() {
    let bindings = bindgen::Builder::default()
        .header("c/include/mun_abi.h")
        .whitelist_type("Mun.*")
        .blacklist_type("MunPrivacy.*")
        .parse_callbacks(Box::new(RemoveVendorName))
        .raw_line("#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]")
        .raw_line("use crate::Privacy;")
        .generate()
        .expect("Unable to generate bindings for 'mun_abi.h'");

    let out_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("src/autogen.rs"))
        .expect(&format!(
            "Couldn't write bindings to '{}'",
            out_path.as_path().to_string_lossy()
        ));
}

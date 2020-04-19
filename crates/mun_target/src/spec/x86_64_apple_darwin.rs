use crate::spec::{LinkerFlavor, Target, TargetResult};

pub fn target() -> TargetResult {
    let mut base = super::apple_base::opts();
    base.cpu = "core2".to_string();

    // Clang automatically chooses a more specific target based on
    // MACOSX_DEPLOYMENT_TARGET.  To enable cross-language LTO to work
    // correctly, we do too.
    let arch = "x86_64";
    let llvm_target = super::apple_base::macos_llvm_target(&arch);

    Ok(Target {
        llvm_target,
        target_os: "macos".to_string(),
        target_endian: "little".to_string(),
        target_pointer_width: "64".to_string(),
        target_c_int_width: "32".to_string(),
        target_env: String::new(),
        target_vendor: "apple".to_string(),
        arch: arch.to_string(),
        data_layout: "e-m:o-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128"
            .to_string(),
        linker_flavor: LinkerFlavor::Ld64,
        options: base,
    })
}

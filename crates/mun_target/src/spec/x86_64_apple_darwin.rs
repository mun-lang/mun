use crate::spec::{Target, TargetOptions};

pub fn target() -> Target {
    // Clang automatically chooses a more specific target based on
    // MACOSX_DEPLOYMENT_TARGET.  To enable cross-language LTO to work
    // correctly, we do too.
    let arch = "x86_64";
    let llvm_target = super::apple_base::macos_llvm_target(arch);

    Target {
        llvm_target,
        pointer_width: 64,
        arch: arch.to_string(),
        data_layout: "e-m:o-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128"
            .to_string(),
        options: TargetOptions {
            cpu: "core2".into(),
            .. super::apple_base::opts("macos")
        },
    }
}

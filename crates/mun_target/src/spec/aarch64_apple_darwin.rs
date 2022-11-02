use crate::spec::{Target, TargetOptions};

pub fn target() -> Target {
    // Clang automatically chooses a more specific target based on
    // MACOSX_DEPLOYMENT_TARGET.  To enable cross-language LTO to work
    // correctly, we do too.
    let arch = "arm64";
    let llvm_target = super::apple_base::ios_llvm_target(arch);
    let (major, minor) = super::apple_base::macos_deployment_target(arch);

    Target {
        llvm_target,
        pointer_width: 64,
        arch: "aarch64".into(),
        data_layout: "e-m:o-i64:64-i128:128-n32:64-S128".into(),
        options: TargetOptions {
            cpu: "apple-a14".into(),
            min_os_version: Some((major, minor, 0)),
            .. super::apple_base::opts("macos")
        },
    }
}
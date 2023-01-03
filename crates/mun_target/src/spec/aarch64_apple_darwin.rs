use crate::spec::{Target, TargetOptions};
use crate::spec::apple_base::{Arch, macos_llvm_target};

pub fn target() -> Target {
    let arch = Arch::Arm64;

    Target {
        // Clang automatically chooses a more specific target based on MACOSX_DEPLOYMENT_TARGET.
        // To enable cross-language LTO to work correctly, we do too.
        llvm_target: macos_llvm_target(arch).into(),
        pointer_width: 64,
        arch: arch.target_arch()    ,
        data_layout: "e-m:o-i64:64-i128:128-n32:64-S128".into(),
        options: TargetOptions {
            cpu: "apple-a14".into(),
            .. super::apple_base::opts("macos", arch)
        },
    }
}

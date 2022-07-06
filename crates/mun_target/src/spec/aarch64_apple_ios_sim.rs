use crate::spec::{Target, TargetOptions};
use super::apple_sdk_base::{opts, Arch};

pub fn target() -> Target {
    // Clang automatically chooses a more specific target based on
    // IPHONEOS_DEPLOYMENT_TARGET.
    // This is required for the target to pick the right
    // MACH-O commands, so we do too.
    let arch = "arm64";
    let llvm_target = super::apple_base::ios_sim_llvm_target(arch);
    let (major, minor) = super::apple_base::ios_deployment_target();

    Target {
        llvm_target,
        pointer_width: 64,
        arch: "aarch64".into(),
        data_layout: "e-m:o-i64:64-i128:128-n32:64-S128".into(),
        options: TargetOptions {
            features: "+neon,+fp-armv8,+apple-a7".into(),
            min_os_version: Some((major, minor, 0)),
            ..opts("ios", Arch::Arm64_sim)
        }
    }
}

use crate::spec::{Target, TargetOptions};
use crate::spec::apple_base::{Arch, ios_sim_llvm_target, opts};

pub fn target() -> Target {
    let arch = Arch::Arm64_sim;
    Target {
        // Clang automatically chooses a more specific target based on
        // IPHONEOS_DEPLOYMENT_TARGET.
        // This is required for the target to pick the right
        // MACH-O commands, so we do too.
        llvm_target: ios_sim_llvm_target(arch).into(),
        pointer_width: 64,
        data_layout: "e-m:o-i64:64-i128:128-n32:64-S128".into(),
        arch: arch.target_arch(),
        options: TargetOptions {
            features: "+neon,+fp-armv8,+apple-a7".into(),
            ..opts("ios", arch)
        }
    }
}

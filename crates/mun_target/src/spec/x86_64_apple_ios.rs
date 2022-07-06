use crate::spec::{Target, TargetOptions};
use super::apple_sdk_base::{opts, Arch};

pub fn target() -> Target {
    let llvm_target = super::apple_base::ios_sim_llvm_target("x86_64");
    let (major, minor) = super::apple_base::ios_deployment_target();

    Target {
        llvm_target,
        pointer_width: 64,
        arch: "x86_64".into(),
        data_layout: "e-m:o-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128".into(),
        options: TargetOptions {
            min_os_version: Some((major, minor, 0)),
            ..opts("ios", Arch::X86_64)
        }
    }
}

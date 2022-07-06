use crate::spec::{LinkerFlavor, TargetOptions};
use std::env;

pub fn opts(os: &'static str) -> TargetOptions {
    TargetOptions {
        os: os.into(),
        vendor: "apple".into(),
        linker_flavor: LinkerFlavor::Ld64,
        dll_prefix: "lib".to_string(),
        ..Default::default()
    }
}

fn deployment_target(var_name: &str) -> Option<(u32, u32)> {
    let deployment_target = env::var(var_name).ok();
    deployment_target
        .as_ref()
        .and_then(|s| s.split_once('.'))
        .and_then(|(a, b)| {
            a.parse::<u32>()
                .and_then(|a| b.parse::<u32>().map(|b| (a, b)))
                .ok()
        })
}

fn macos_default_deployment_target(arch: &str) -> (u32, u32) {
    if arch == "arm64" {
        (11, 0)
    } else {
        (10, 7)
    }
}

pub fn macos_deployment_target(arch: &str) -> (u32, u32) {
    deployment_target("MACOSX_DEPLOYMENT_TARGET")
        .unwrap_or_else(|| macos_default_deployment_target(arch))
}

pub fn macos_llvm_target(arch: &str) -> String {
    let (major, minor) = macos_deployment_target(arch);
    format!("{}-apple-macosx{}.{}.0", arch, major, minor)
}

pub fn ios_deployment_target() -> (u32, u32) {
    deployment_target("IPHONEOS_DEPLOYMENT_TARGET").unwrap_or((7, 0))
}

pub fn ios_llvm_target(arch: &str) -> String {
    // Modern iOS tooling extracts information about deployment target
    // from LC_BUILD_VERSION. This load command will only be emitted when
    // we build with a version specific `llvm_target`, with the version
    // set high enough. Luckily one LC_BUILD_VERSION is enough, for Xcode
    // to pick it up (since std and core are still built with the fallback
    // of version 7.0 and hence emit the old LC_IPHONE_MIN_VERSION).
    let (major, minor) = ios_deployment_target();
    format!("{}-apple-ios{}.{}.0", arch, major, minor)
}

pub fn ios_sim_llvm_target(arch: &str) -> String {
    let (major, minor) = ios_deployment_target();
    format!("{}-apple-ios{}.{}.0-simulator", arch, major, minor)
}

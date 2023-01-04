use crate::spec::{LinkerFlavor, TargetOptions};
use std::borrow::Cow;
use std::env;

use Arch::*;

#[allow(non_camel_case_types, dead_code)]
#[derive(Copy, Clone)]
pub enum Arch {
    Armv7,
    Armv7k,
    Armv7s,
    Arm64,
    Arm64_32,
    I386,
    I686,
    X86_64,
    X86_64_sim,
    X86_64_macabi,
    Arm64_macabi,
    Arm64_sim,
}

impl Arch {
    pub fn target_name(self) -> &'static str {
        match self {
            Armv7 => "armv7",
            Armv7k => "armv7k",
            Armv7s => "armv7s",
            Arm64 | Arm64_macabi | Arm64_sim => "arm64",
            Arm64_32 => "arm64_32",
            I386 => "i386",
            I686 => "i686",
            X86_64 | X86_64_sim | X86_64_macabi => "x86_64",
        }
    }

    pub fn target_arch(self) -> Cow<'static, str> {
        Cow::Borrowed(match self {
            Armv7 | Armv7k | Armv7s => "arm",
            Arm64 | Arm64_32 | Arm64_macabi | Arm64_sim => "aarch64",
            I386 | I686 => "x86",
            X86_64 | X86_64_sim | X86_64_macabi => "x86_64",
        })
    }

    fn target_abi(self) -> &'static str {
        match self {
            Armv7 | Armv7k | Armv7s | Arm64 | Arm64_32 | I386 | I686 | X86_64 => "",
            X86_64_macabi | Arm64_macabi => "macabi",
            // x86_64-apple-ios is a simulator target, even though it isn't
            // declared that way in the target like the other ones...
            Arm64_sim | X86_64_sim => "sim",
        }
    }

    fn target_cpu(self) -> &'static str {
        match self {
            Armv7 => "cortex-a8", // iOS7 is supported on iPhone 4 and higher
            Armv7k => "cortex-a8",
            Armv7s => "cortex-a9",
            Arm64 => "apple-a7",
            Arm64_32 => "apple-s4",
            I386 | I686 => "yonah",
            X86_64 | X86_64_sim => "core2",
            X86_64_macabi => "core2",
            Arm64_macabi => "apple-a12",
            Arm64_sim => "apple-a12",
        }
    }
}

fn pre_link_args(os: &'static str, arch: Arch, abi: &'static str) -> Vec<Cow<'static, str>> {
    let platform_name: Cow<'static, str> = match abi {
        "sim" => format!("{}-simulator", os).into(),
        "macabi" => "mac-catalyst".into(),
        _ => os.into(),
    };

    let platform_version: Cow<'static, str> = match os {
        "ios" => ios_lld_platform_version(),
        "tvos" => tvos_lld_platform_version(),
        "watchos" => watchos_lld_platform_version(),
        "macos" => macos_lld_platform_version(arch),
        _ => unreachable!(),
    }
    .into();

    let arch = arch.target_name();

    vec![
        Cow::Borrowed("-arch"),
        Cow::Borrowed(arch),
        Cow::Borrowed("-platform_version"),
        platform_name,
        platform_version.clone(),
        platform_version,
    ]
}

pub fn opts(os: &'static str, arch: Arch) -> TargetOptions {
    let abi = arch.target_abi();
    TargetOptions {
        abi: abi.into(),
        os: os.into(),
        cpu: arch.target_cpu().into(),
        vendor: "apple".into(),
        linker_flavor: LinkerFlavor::Ld64,
        dll_prefix: "lib".to_string(),
        is_like_osx: os == "macos",
        pre_link_args: pre_link_args(os, arch, abi),
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

fn macos_default_deployment_target(arch: Arch) -> (u32, u32) {
    // Note: Arm64_sim is not included since macOS has no simulator.
    if matches!(arch, Arm64 | Arm64_macabi) {
        (11, 0)
    } else {
        (10, 7)
    }
}

fn macos_deployment_target(arch: Arch) -> (u32, u32) {
    deployment_target("MACOSX_DEPLOYMENT_TARGET")
        .unwrap_or_else(|| macos_default_deployment_target(arch))
}

fn macos_lld_platform_version(arch: Arch) -> String {
    let (major, minor) = macos_deployment_target(arch);
    format!("{}.{}", major, minor)
}

pub fn macos_llvm_target(arch: Arch) -> String {
    let (major, minor) = macos_deployment_target(arch);
    format!("{}-apple-macosx{}.{}.0", arch.target_name(), major, minor)
}

pub fn ios_deployment_target() -> (u32, u32) {
    deployment_target("IPHONEOS_DEPLOYMENT_TARGET").unwrap_or((7, 0))
}

pub fn ios_llvm_target(arch: Arch) -> String {
    // Modern iOS tooling extracts information about deployment target
    // from LC_BUILD_VERSION. This load command will only be emitted when
    // we build with a version specific `llvm_target`, with the version
    // set high enough. Luckily one LC_BUILD_VERSION is enough, for Xcode
    // to pick it up (since std and core are still built with the fallback
    // of version 7.0 and hence emit the old LC_IPHONE_MIN_VERSION).
    let (major, minor) = ios_deployment_target();
    format!("{}-apple-ios{}.{}.0", arch.target_name(), major, minor)
}

pub fn ios_sim_llvm_target(arch: Arch) -> String {
    let (major, minor) = ios_deployment_target();
    format!(
        "{}-apple-ios{}.{}.0-simulator",
        arch.target_name(),
        major,
        minor
    )
}

fn ios_lld_platform_version() -> String {
    let (major, minor) = ios_deployment_target();
    format!("{}.{}", major, minor)
}

fn tvos_deployment_target() -> (u32, u32) {
    deployment_target("TVOS_DEPLOYMENT_TARGET").unwrap_or((7, 0))
}

fn tvos_lld_platform_version() -> String {
    let (major, minor) = tvos_deployment_target();
    format!("{}.{}", major, minor)
}

fn watchos_deployment_target() -> (u32, u32) {
    deployment_target("WATCHOS_DEPLOYMENT_TARGET").unwrap_or((5, 0))
}

fn watchos_lld_platform_version() -> String {
    let (major, minor) = watchos_deployment_target();
    format!("{}.{}", major, minor)
}

// pub fn watchos_sim_llvm_target(arch: Arch) -> String {
//     let (major, minor) = watchos_deployment_target();
//     format!("{}-apple-watchos{}.{}.0-simulator", arch.target_name(), major, minor)
// }

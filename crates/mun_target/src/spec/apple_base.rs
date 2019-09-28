use crate::spec::TargetOptions;
use std::env;

pub fn opts() -> TargetOptions {
    TargetOptions {
        dll_prefix: "lib".to_string(),
        dll_suffix: ".dylib".to_string(),
        ..Default::default()
    }
}

fn macos_deployment_target() -> (u32, u32) {
    let deployment_target = env::var("MACOSX_DEPLOYMENT_TARGET").ok();
    let version = deployment_target
        .as_ref()
        .and_then(|s| {
            let mut i = s.splitn(2, '.');
            i.next().and_then(|a| i.next().map(|b| (a, b)))
        })
        .and_then(|(a, b)| {
            a.parse::<u32>()
                .and_then(|a| b.parse::<u32>().map(|b| (a, b)))
                .ok()
        });

    version.unwrap_or((10, 7))
}

pub fn macos_llvm_target(arch: &str) -> String {
    let (major, minor) = macos_deployment_target();
    format!("{}-apple-macosx{}.{}.0", arch, major, minor)
}

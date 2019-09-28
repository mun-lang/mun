use crate::spec::{LinkerFlavor, Target, TargetResult};

pub fn target() -> TargetResult {
    let mut base = super::linux_base::opts();
    base.cpu = "x86-64".to_string();

    Ok(Target {
        llvm_target: "x86_64-unknown-linux-gnu".to_string(),
        target_os: "linux".to_string(),
        target_env: "gnu".to_string(),
        target_vendor: "unknown".to_string(),
        arch: "x86_64".to_string(),
        linker_flavor: LinkerFlavor::Ld,
        options: base,
    })
}

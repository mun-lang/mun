use crate::spec::{LinkerFlavor, Target, TargetResult};

pub fn target() -> TargetResult {
    let mut base = super::apple_base::opts();
    base.cpu = "core2".to_string();

    // Clang automatically chooses a more specific target based on
    // MACOSX_DEPLOYMENT_TARGET.  To enable cross-language LTO to work
    // correctly, we do too.
    let arch = "x86_64";
    let llvm_target = super::apple_base::macos_llvm_target(&arch);

    Ok(Target {
        llvm_target,
        target_os: "macos".to_string(),
        target_env: String::new(),
        target_vendor: "apple".to_string(),
        arch: arch.to_string(),
        linker_flavor: LinkerFlavor::Ld64,
        options: base,
    })
}

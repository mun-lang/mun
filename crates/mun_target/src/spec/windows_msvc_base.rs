use crate::spec::{LinkerFlavor, TargetOptions};

pub fn opts() -> TargetOptions {
    TargetOptions {
        os: "windows".into(),
        env: "msvc".into(),
        vendor: "pc".into(),
        linker_flavor: LinkerFlavor::Msvc,
        dll_prefix: "".to_string(),
        is_like_windows: true,
        is_like_msvc: true,
        ..Default::default()
    }
}

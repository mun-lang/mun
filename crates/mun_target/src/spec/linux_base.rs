use crate::spec::{LinkerFlavor, TargetOptions};

pub fn opts() -> TargetOptions {
    TargetOptions {
        os: "linux".to_string(),
        env: "gnu".to_string(),
        vendor: "unknown".to_string(),
        linker_flavor: LinkerFlavor::Ld,
        ..Default::default()
    }
}

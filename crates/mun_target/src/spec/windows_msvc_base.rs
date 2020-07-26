use crate::spec::TargetOptions;

pub fn opts() -> TargetOptions {
    TargetOptions {
        dll_prefix: "".to_string(),
        is_like_windows: true,
        is_like_msvc: true,
        ..Default::default()
    }
}

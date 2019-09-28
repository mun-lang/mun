use crate::spec::TargetOptions;

pub fn opts() -> TargetOptions {
    TargetOptions {
        dll_prefix: "".to_string(),
        dll_suffix: ".dll".to_string(),
        is_like_windows: true,
        ..Default::default()
    }
}

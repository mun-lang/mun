use crate::spec::TargetOptions;

pub fn opts() -> TargetOptions {
    TargetOptions {
        ..Default::default()
    }
}

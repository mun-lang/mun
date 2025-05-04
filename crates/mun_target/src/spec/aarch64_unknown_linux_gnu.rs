use crate::spec::{Target, TargetOptions};

pub fn target() -> Target {
    Target {
        llvm_target: "aarch64-unknown-linux-gnu".into(),
        pointer_width: 64,
        arch: "aarch64".into(),
        data_layout: "e-m:e-i8:8:32-i16:16:32-i64:64-i128:128-n32:64-S128".into(),
        options: TargetOptions {
            cpu: "apple-a14".into(),
            ..super::linux_base::opts()
        },
    }
}

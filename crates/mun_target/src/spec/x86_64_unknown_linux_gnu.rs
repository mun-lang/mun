use crate::spec::{Target, TargetOptions};

pub fn target() -> Target {
    Target {
        llvm_target: "x86_64-unknown-linux-gnu".to_string(),
        pointer_width: 64,
        arch: "x86_64".to_string(),
        data_layout: "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128"
            .to_string(),
        options: TargetOptions {
            cpu: "x86_64".into(),
            .. super::linux_base::opts()
        },
    }
}

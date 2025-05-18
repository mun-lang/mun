use crate::spec::Target;

pub fn target() -> Target {
    Target {
        llvm_target: "aarch64-pc-windows-msvc".into(),
        pointer_width: 64,
        arch: "aarch64".into(),
        data_layout: "e-m:w-p:64:64-i32:32-i64:64-i128:128-n32:64-S128".into(),
        options: super::windows_msvc_base::opts(),
    }
}

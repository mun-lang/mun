mod apple_base;
mod linux_base;
mod windows_msvc_base;

use std::borrow::Cow;

use crate::{abi::Endian, host_triple};

#[derive(Debug, Clone, Copy, Eq, Ord, PartialOrd, PartialEq, Hash)]
pub enum LinkerFlavor {
    Ld,
    Ld64,
    Msvc,
}

/// Everything Mun knows about a target.
/// Every field must be specified, there are no default values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Target {
    /// Target triple to pass to LLVM
    pub llvm_target: Cow<'static, str>,

    /// String to use as the `target_pointer_width` `cfg` variable.
    pub pointer_width: u32,

    /// The name of the architecture. For example "x86" or "`x86_64`", "arm",
    /// "aarch64"
    pub arch: Cow<'static, str>,

    /// [Data layout](http://llvm.org/docs/LangRef.html#data-layout) to pass to LLVM.
    pub data_layout: Cow<'static, str>,

    /// Optional settings
    pub options: TargetOptions,
}

/// Optional aspects of target specification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TargetOptions {
    /// True if this is a built-in target
    pub is_builtin: bool,

    /// Used as the `target_endian` `cfg` variable. Defaults to little endian.
    pub endian: Endian,

    /// Width of `c_int` type
    pub c_int_width: String,

    /// The name of the OS
    pub os: String,

    /// The name of the environment
    pub env: String,

    /// ABI name to distinguish multiple ABIs on the same OS and architecture.
    /// For instance, `"eabi"` or `"eabihf"`. Defaults to "".
    pub abi: String,

    /// The name of the vendor
    pub vendor: String,

    /// Linker flavor
    pub linker_flavor: LinkerFlavor,

    /// Linker arguments that are passed *before* any user-defined libraries.
    pub pre_link_args: Vec<Cow<'static, str>>,

    /// Default CPU to pass to LLVM. Corresponds to `llc -mcpu=$cpu`. Defaults
    /// to "generic".
    pub cpu: String,

    /// Default target features to pass to LLVM. These features will *always* be
    /// passed, and cannot be disabled even via `-C`. Corresponds to `llc
    /// -mattr=$features`.
    pub features: String,

    /// String to prepend to the name of every dynamic library. Defaults to
    /// "lib".
    pub dll_prefix: String,

    /// Whether the target toolchain is like Windows
    pub is_like_windows: bool,
    pub is_like_msvc: bool,

    /// Whether the target toolchain is like macOS's. Only useful for compiling
    /// against iOS/macOS, in particular running dsymutil and some other
    /// stuff like `-dead_strip`. Defaults to false.
    pub is_like_osx: bool,
}

impl Default for TargetOptions {
    fn default() -> Self {
        TargetOptions {
            is_builtin: false,
            endian: Endian::Little,
            c_int_width: "32".into(),
            os: "none".into(),
            env: "".into(),
            abi: "".into(),
            vendor: "unknown".into(),
            linker_flavor: LinkerFlavor::Ld,
            pre_link_args: vec![],
            cpu: "generic".to_string(),
            features: "".to_string(),
            dll_prefix: "lib".to_string(),
            is_like_windows: false,
            is_like_msvc: false,
            is_like_osx: false,
        }
    }
}

macro_rules! supported_targets {
    ( $(($( $triple:literal, )+ $module:ident ),)+ ) => {
        $ ( mod $ module; ) +

        /// List of supported targets
        const TARGETS: &[&str] = &[$($($triple),+),+];

        fn load_specific(target: &str) -> Option<Target> {
            let mut t = match target {
                $( $($triple)|+ => $module::target(), )+
                _ => return None,
            };
            t.options.is_builtin = true;
            log::debug!("got builtin target: {:?}", t);
            Some(t)
        }

        pub fn get_targets() -> impl Iterator<Item = &'static str> {
            TARGETS.iter().copied()
        }
    }
}

supported_targets!(
    ("x86_64-apple-darwin", x86_64_apple_darwin),
    ("x86_64-apple-ios", x86_64_apple_ios),
    ("x86_64-pc-windows-msvc", x86_64_pc_windows_msvc),
    ("x86_64-unknown-linux-gnu", x86_64_unknown_linux_gnu),
    ("aarch64-apple-darwin", aarch64_apple_darwin),
    ("aarch64-apple-ios", aarch64_apple_ios),
    ("aarch64-apple-ios-sim", aarch64_apple_ios_sim),
    ("aarch64-unknown-linux-gnu", aarch64_unknown_linux_gnu),
);

impl Target {
    pub fn search(target_triple: &str) -> Option<Target> {
        load_specific(target_triple)
    }

    pub fn host_target() -> Option<Target> {
        Self::search(host_triple())
    }
}

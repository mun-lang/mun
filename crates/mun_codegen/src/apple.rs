use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::{
    env, io,
    path::{Path, PathBuf},
    process::Command,
};

/// Finds the Apple SDK root directory by checking the `SDKROOT` environment variable or running
/// `xcrun --show-sdk-path`. The result is cached so multiple calls to this function should be
/// fast.
pub fn get_apple_sdk_root(sdk_name: &str) -> Result<PathBuf, String> {
    static SDK_PATH: Lazy<Mutex<HashMap<String, PathBuf>>> = Lazy::new(Default::default);

    let mut lock = SDK_PATH.lock();
    if let Some(path) = lock.get(sdk_name) {
        return Ok(path.clone());
    }

    match find_apple_sdk_root(sdk_name) {
        Ok(sdk_root) => Ok(lock.entry(sdk_name.into()).or_insert(sdk_root).clone()),
        Err(e) => Err(e),
    }
}

fn find_apple_sdk_root(sdk_name: &str) -> Result<PathBuf, String> {
    // Following what clang (and rustc) does
    // (https://github.com/llvm/llvm-project/blob/
    // 296a80102a9b72c3eda80558fb78a3ed8849b341/clang/lib/Driver/ToolChains/Darwin.cpp#L1661-L1678
    // and https://github.dev/rust-lang/rust/blob/33eb3c05c54b306afea341dd233671a9f039156f/compiler/
    // rustc_codegen_ssa/src/back/link.rs#L2659-L2681)
    // to allow the SDK path to be set. (For clang, xcrun sets
    // SDKROOT; for rustc, the user or build system can set it, or we
    // can fall back to checking for xcrun on PATH.)

    if let Ok(sdkroot) = env::var("SDKROOT") {
        let p = PathBuf::from(&sdkroot);
        match sdk_name {
            // Ignore `SDKROOT` if it's clearly set for the wrong platform.
            "appletvos"
                if sdkroot.contains("TVSimulator.platform")
                    || sdkroot.contains("MacOSX.platform") => {}
            "appletvsimulator"
                if sdkroot.contains("TVOS.platform") || sdkroot.contains("MacOSX.platform") => {}
            "iphoneos"
                if sdkroot.contains("iPhoneSimulator.platform")
                    || sdkroot.contains("MacOSX.platform") => {}
            "iphonesimulator"
                if sdkroot.contains("iPhoneOS.platform") || sdkroot.contains("MacOSX.platform") => {
            }
            "macosx10.15"
                if sdkroot.contains("iPhoneOS.platform")
                    || sdkroot.contains("iPhoneSimulator.platform") => {}
            "watchos"
                if sdkroot.contains("WatchSimulator.platform")
                    || sdkroot.contains("MacOSX.platform") => {}
            "watchsimulator"
                if sdkroot.contains("WatchOS.platform") || sdkroot.contains("MacOSX.platform") => {}
            // Ignore `SDKROOT` if it's not a valid path.
            _ if !p.is_absolute() || p == Path::new("/") || !p.exists() => {}
            _ => return Ok(p),
        }
    }

    let res = Command::new("xcrun")
        .arg("--show-sdk-path")
        .arg("-sdk")
        .arg(sdk_name)
        .output()
        .and_then(|output| {
            if output.status.success() {
                Ok(String::from_utf8(output.stdout).unwrap())
            } else {
                let error = String::from_utf8(output.stderr);
                let error = format!("process exit with error: {}", error.unwrap());
                Err(io::Error::new(io::ErrorKind::Other, &error[..]))
            }
        });

    match res {
        Ok(output) => Ok(PathBuf::from(output.trim())),
        Err(e) => Err(format!("failed to get SDK path: {}", e)),
    }
}

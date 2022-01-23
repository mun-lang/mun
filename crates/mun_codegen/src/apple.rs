use once_cell::sync::OnceCell;
use std::{
    env, io,
    path::{Path, PathBuf},
    process::Command,
};

/// Finds the Apple SDK root directory by checking the `SDKROOT` environment variable or running
/// `xcrun --show-sdk-path`. The result is cached so multiple calls to this function should be
/// fast.
pub fn get_apple_sdk_root() -> Result<&'static Path, String> {
    static SDK_PATH: OnceCell<PathBuf> = OnceCell::new();

    SDK_PATH
        .get_or_try_init(|| {
            if let Ok(sdkroot) = env::var("SDKROOT") {
                return Ok(PathBuf::from(sdkroot));
            }

            let res = Command::new("xcrun")
                .arg("--show-sdk-path")
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
        })
        .map(AsRef::as_ref)
}

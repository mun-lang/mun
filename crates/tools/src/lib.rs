pub type Result<T> = std::result::Result<T, failure::Error>;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
pub use teraron::{Mode, Overwrite, Verify};

pub mod abi;
pub mod runtime_capi;
pub mod syntax;

/// A helper to update file on disk if it has changed.
/// With verify = false,
fn update(path: &Path, contents: &str, mode: Mode) -> Result<()> {
    let old_contents = fs::read_to_string(path)?;
    let old_contents = old_contents.replace("\r\n", "\n");
    let contents = contents.replace("\r\n", "\n");
    if old_contents == contents {
        return Ok(());
    }

    if mode == Mode::Verify {
        let changes = difference::Changeset::new(&old_contents, &contents, "\n");
        failure::bail!("`{}` is not up-to-date:\n{}", path.display(), changes);
    }
    eprintln!("updating {}", path.display());
    fs::write(path, contents)?;
    Ok(())
}

fn reformat(text: impl std::fmt::Display) -> Result<String> {
    let mut rustfmt = Command::new("rustfmt")
        //.arg("--config-path")
        //.arg(project_root().join("rustfmt.toml"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    write!(rustfmt.stdin.take().unwrap(), "{}", text)?;
    let output = rustfmt.wait_with_output()?;
    let stdout = String::from_utf8(output.stdout)?;
    let preamble = "Generated file, do not edit by hand, see `crate/ra_tools/src/codegen`";
    Ok(format!("//! {}\n\n{}", preamble, stdout))
}

pub fn project_root() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf()
}

#[cfg(test)]
mod tests {
    use crate::Mode;

    #[test]
    fn grammar_is_fresh() {
        if let Err(error) = super::syntax::generate(Mode::Verify) {
            panic!(
                "Please update syntax by running `cargo gen-syntax`, its out of date.\n{}",
                error
            );
        }
    }

    #[test]
    fn runtime_capi_is_fresh() {
        if let Err(error) = super::runtime_capi::generate(Mode::Verify) {
            panic!(
                "Please update runtime-capi by running `cargo gen-runtime-capi`, its out of date.\n{}",
                error
            );
        }
    }

    #[test]
    fn abi_is_fresh() {
        if let Err(error) = super::abi::generate(Mode::Verify) {
            panic!(
                "Please update abi by running `cargo gen-abi`, its out of date.\n{}",
                error
            );
        }
    }
}

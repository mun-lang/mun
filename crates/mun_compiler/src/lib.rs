#![allow(clippy::enum_variant_names)] // This is a HACK because we use salsa

mod db;
///! This library contains the code required to go from source code to binaries.
pub mod diagnostics;
mod diagnostics_snippets;
mod driver;

pub use hir::FileId;
pub use mun_target::spec::Target;
pub use paths::{RelativePath, RelativePathBuf};
use std::path::{Path, PathBuf};

pub use crate::driver::DisplayColor;
pub use crate::driver::{Config, Driver};
pub use mun_codegen::OptimizationLevel;

pub use crate::db::CompilerDatabase;
pub use annotate_snippets::snippet::AnnotationType;
use mun_project::Package;
use std::ffi::OsStr;
use std::io::stderr;

#[derive(Debug, Clone)]
pub enum PathOrInline {
    Path(PathBuf),
    Inline {
        rel_path: RelativePathBuf,
        contents: String,
    },
}

#[derive(Debug, Clone)]
pub struct CompilerOptions {
    /// The input for the compiler
    pub input: PathOrInline,

    /// The compiler configuration
    pub config: Config,
}

impl CompilerOptions {
    pub fn with_path<P: AsRef<Path>>(input: P) -> CompilerOptions {
        CompilerOptions {
            input: PathOrInline::Path(input.as_ref().to_path_buf()),
            config: Config::default(),
        }
    }

    pub fn with_file<P: Into<RelativePathBuf>, T: AsRef<str>>(
        path: P,
        input: T,
    ) -> CompilerOptions {
        CompilerOptions {
            input: PathOrInline::Inline {
                rel_path: path.into(),
                contents: input.as_ref().to_string(),
            },
            config: Config::default(),
        }
    }
}

/// Returns true if the given path is considered to be a Mun source file
pub fn is_source_file<P: AsRef<Path>>(p: P) -> bool {
    p.as_ref().extension() == Some(&OsStr::new("mun"))
}

/// Returns and creates the output dir for the specified package
pub fn ensure_package_output_dir(
    package: &Package,
    config: &Config,
) -> Result<PathBuf, anyhow::Error> {
    let out_dir = config
        .out_dir
        .clone()
        .unwrap_or_else(|| package.root().join("target"));
    std::fs::create_dir_all(&out_dir)?;
    Ok(out_dir)
}

pub fn compile_manifest(manifest_path: &Path, config: Config) -> Result<bool, anyhow::Error> {
    let (_package, mut driver) = Driver::with_package_path(manifest_path, config)?;

    // Emit diagnostics. If one of the snippets is an error, abort gracefully.
    if driver.emit_diagnostics(&mut stderr())? {
        return Ok(false);
    };

    // Write out all assemblies
    driver.write_all_assemblies(false)?;

    Ok(true)
}

/// Determines the relative path of a file to the source directory.
pub fn compute_source_relative_path(
    source_dir: &Path,
    source_path: &Path,
) -> Result<RelativePathBuf, anyhow::Error> {
    RelativePathBuf::from_path(source_path.strip_prefix(source_dir).map_err(|e| {
        anyhow::anyhow!(
            "could not determine relative source path for '{}': {}",
            source_path.display(),
            e
        )
    })?)
    .map_err(|e| {
        anyhow::anyhow!(
            "could not determine source relative path for '{}': {}",
            source_path.display(),
            e
        )
    })
}

#[cfg(test)]
mod test {
    use crate::{compute_source_relative_path, is_source_file, RelativePath};
    use std::path::Path;

    #[test]
    fn test_is_source_file() {
        assert_eq!(is_source_file("main.mun"), true);
        assert_eq!(is_source_file("foo.mun"), true);
        assert_eq!(is_source_file("foo/bar.mun"), true);
        assert_eq!(is_source_file("foo/bar"), false);
    }

    #[test]
    fn test_compute_source_relative_path() {
        let source_dir = Path::new("some_path/src");
        assert_eq!(
            compute_source_relative_path(&source_dir, &source_dir.join("main.mun")).unwrap(),
            RelativePath::new("main.mun")
        );
        assert_eq!(
            compute_source_relative_path(&source_dir, &source_dir.join("foo/bar/main.mun"))
                .unwrap(),
            RelativePath::new("foo/bar/main.mun")
        );
    }
}

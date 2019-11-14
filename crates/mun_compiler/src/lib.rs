#![allow(clippy::enum_variant_names)] // This is a HACK because we use salsa

mod db;
///! This library contains the code required to go from source code to binaries.
mod diagnostics;
mod driver;

pub use mun_hir::{RelativePath, RelativePathBuf};
pub use mun_target::spec::Target;
use std::path::{Path, PathBuf};
pub use termcolor::{ColorChoice, StandardStream};

pub use crate::driver::{Config, Driver};
pub use mun_codegen::OptimizationLevel;

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

/// Returns the target triple of the host machine. This can be used as a default target.
pub fn host_triple() -> &'static str {
    // Get the host triple out of the build environment. This ensures that our
    // idea of the host triple is the same as for the set of libraries we've
    // actually built.  We can't just take LLVM's host triple because they
    // normalize all ix86 architectures to i386.
    //
    // Instead of grabbing the host triple (for the current host), we grab (at
    // compile time) the target triple that this rustc is built with and
    // calling that (at runtime) the host triple.
    (option_env!("CFG_COMPILER_HOST_TRIPLE")).expect("CFG_COMPILER_HOST_TRIPLE")
}

pub fn main(options: CompilerOptions) -> Result<Option<PathBuf>, failure::Error> {
    let (driver, file_id) = Driver::with_file(options.config, options.input)?;

    let mut writer = StandardStream::stderr(ColorChoice::Auto);
    if driver.emit_diagnostics(&mut writer)? {
        Ok(None)
    } else {
        driver.write_assembly(file_id)
    }
}

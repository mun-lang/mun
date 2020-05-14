#![allow(clippy::enum_variant_names)] // This is a HACK because we use salsa
mod annotate;
mod db;
///! This library contains the code required to go from source code to binaries.
mod diagnostics;
mod diagnostics_snippets;
mod driver;

pub use mun_hir::{FileId, RelativePath, RelativePathBuf};
pub use mun_target::spec::Target;
use std::path::{Path, PathBuf};

pub use crate::driver::DisplayColor;
pub use crate::driver::{Config, Driver};
pub use annotate::{AnnotationBuilder, SliceBuilder, SnippetBuilder};
pub use mun_codegen::OptimizationLevel;

use anyhow::Result;
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

pub fn main(options: CompilerOptions) -> Result<Option<PathBuf>> {
    let (mut driver, file_id) = Driver::with_file(options.config, options.input)?;

    if driver.emit_diagnostics(&mut stderr())? {
        Ok(None)
    } else {
        driver.write_assembly(file_id).map(Some)
    }
}

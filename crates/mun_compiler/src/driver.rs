//! `Driver` is a stateful compiler frontend that enables incremental compilation by retaining state
//! from previous compilation.

use crate::{
    db::CompilerDatabase,
    diagnostics::{diagnostics, Emit},
    PathOrInline,
};
use mun_codegen::IrDatabase;
use mun_hir::{FileId, RelativePathBuf, SourceDatabase, SourceRoot, SourceRootId};
use mun_target::spec::Target;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

mod config;

pub use self::config::Config;
use mun_errors::{Diagnostic, Level};
use termcolor::WriteColor;

pub const WORKSPACE: SourceRootId = SourceRootId(0);

#[derive(Debug)]
pub struct Driver {
    db: CompilerDatabase,
    out_dir: Option<PathBuf>,
}

impl Driver {
    /// Constructs a driver with a specific configuration.
    pub fn with_config(config: Config) -> Self {
        let mut driver = Driver {
            db: CompilerDatabase::new(),
            out_dir: None,
        };

        // Move relevant configuration into the database
        driver.db.set_target(config.target);
        driver
            .db
            .set_context(Arc::new(mun_codegen::Context::create()));
        driver.db.set_optimization_lvl(config.optimization_lvl);

        driver.out_dir = config.out_dir;

        driver
    }

    /// Constructs a driver with a configuration and a single file.
    pub fn with_file(
        config: Config,
        path: PathOrInline,
    ) -> Result<(Driver, FileId), failure::Error> {
        let mut driver = Driver::with_config(config);

        // Construct a SourceRoot
        let mut source_root = SourceRoot::default();

        // Get the path and contents of the path
        let (rel_path, text) = match path {
            PathOrInline::Path(p) => {
                let filename = p.file_name().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Input path is missing a filename.",
                    )
                })?;
                (
                    RelativePathBuf::from_path(filename).unwrap(),
                    std::fs::read_to_string(p)?,
                )
            }
            PathOrInline::Inline { rel_path, contents } => (rel_path, contents),
        };

        // Store the file information in the database together with the source root
        let file_id = FileId(0);
        driver.db.set_file_relative_path(file_id, rel_path.clone());
        driver.db.set_file_text(file_id, Arc::new(text));
        driver.db.set_file_source_root(file_id, WORKSPACE);
        source_root.insert_file(rel_path, file_id);
        driver.db.set_source_root(WORKSPACE, Arc::new(source_root));

        Ok((driver, file_id))
    }
}

impl Driver {
    /// Sets the contents of a specific file.
    pub fn set_file_text<T: AsRef<str>>(&mut self, file_id: FileId, text: T) {
        self.db
            .set_file_text(file_id, Arc::new(text.as_ref().to_owned()));
    }
}

impl Driver {
    /// Returns a vector containing all the diagnostic messages for the project.
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.db
            .source_root(WORKSPACE)
            .files()
            .map(|f| diagnostics(&self.db, f))
            .flatten()
            .collect()
    }

    /// Emits all diagnostic messages currently in the database; returns true if errors were
    /// emitted.
    pub fn emit_diagnostics(&self, writer: &mut impl WriteColor) -> Result<bool, failure::Error> {
        let mut has_errors = false;
        for file_id in self.db.source_root(WORKSPACE).files() {
            let diags = diagnostics(&self.db, file_id);
            for diagnostic in diags.iter() {
                diagnostic.emit(writer, &self.db, file_id)?;
                if diagnostic.level == Level::Error {
                    has_errors = true;
                }
            }
        }
        Ok(has_errors)
    }
}

impl Driver {
    /// Computes the output path for the assembly of the specified file.
    fn assembly_output_path(&self, file_id: FileId) -> PathBuf {
        let target: Target = self.db.target();
        let relative_path: RelativePathBuf = self.db.file_relative_path(file_id);
        let original_filename = Path::new(relative_path.file_name().unwrap());

        // Get the dll suffix without the starting dot
        let dll_extension = if target.options.dll_suffix.starts_with('.') {
            &target.options.dll_suffix[1..]
        } else {
            &target.options.dll_suffix
        };

        // Add the dll suffix to the original filename
        let output_file_name = original_filename.with_extension(dll_extension);

        // If there is an out dir specified, prepend the output directory
        if let Some(ref out_dir) = self.out_dir {
            out_dir.join(output_file_name)
        } else {
            output_file_name
        }
    }

    /// Generate an assembly for the given file
    pub fn write_assembly(&self, file_id: FileId) -> Result<Option<PathBuf>, failure::Error> {
        let output_path = self.assembly_output_path(file_id);
        mun_codegen::write_module_shared_object(&self.db, file_id, &output_path)?;
        Ok(Some(output_path))
    }
}

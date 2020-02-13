//! `Driver` is a stateful compiler frontend that enables incremental compilation by retaining state
//! from previous compilation.

use crate::{db::CompilerDatabase, diagnostics::diagnostics, PathOrInline};
use mun_codegen::{IrDatabase, ModuleBuilder};
use mun_hir::{FileId, RelativePathBuf, SourceDatabase, SourceRoot, SourceRootId};
use std::{path::PathBuf, sync::Arc};

mod config;

pub use self::config::Config;
use annotate_snippets::{
    display_list::DisplayList,
    formatter::DisplayListFormatter,
    snippet::{AnnotationType, Snippet},
};

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
    pub fn diagnostics(&self) -> Vec<Snippet> {
        self.db
            .source_root(WORKSPACE)
            .files()
            .map(|f| diagnostics(&self.db, f))
            .flatten()
            .collect()
    }

    /// Emits all diagnostic messages currently in the database; returns true if errors were
    /// emitted.
    pub fn emit_diagnostics(
        &self,
        writer: &mut dyn std::io::Write,
    ) -> Result<bool, failure::Error> {
        let mut has_errors = false;
        let dlf = DisplayListFormatter::new(true, false);
        for file_id in self.db.source_root(WORKSPACE).files() {
            let diags = diagnostics(&self.db, file_id);
            for diagnostic in diags {
                let dl = DisplayList::from(diagnostic.clone());
                writeln!(writer, "{}", dlf.format(&dl)).unwrap();
                if let Some(annotation) = diagnostic.title {
                    #[allow(clippy::single_match)]
                    match annotation.annotation_type {
                        AnnotationType::Error => {
                            has_errors = true;
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(has_errors)
    }
}

impl Driver {
    /// Generate an assembly for the given file
    pub fn write_assembly(&mut self, file_id: FileId) -> Result<PathBuf, failure::Error> {
        let module_builder = ModuleBuilder::new(&mut self.db, file_id)?;
        module_builder.finalize(self.out_dir.as_deref())
    }
}

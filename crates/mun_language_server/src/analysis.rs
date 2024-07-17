use std::sync::Arc;

use mun_hir::AstDatabase;
use mun_hir_input::{FileId, LineIndex, PackageId, SourceDatabase};
use mun_syntax::SourceFile;
use salsa::{ParallelDatabase, Snapshot};

use crate::{
    cancelation::Canceled, change::AnalysisChange, completion, db::AnalysisDatabase, diagnostics,
    diagnostics::Diagnostic, file_structure, FilePosition,
};

/// Result of an operation that can be canceled.
pub type Cancelable<T> = Result<T, Canceled>;

/// The `Analysis` struct is the basis of all language server operations. It
/// maintains the current state of the source.
#[derive(Default)]
pub struct Analysis {
    db: AnalysisDatabase,
}

impl Analysis {
    /// Applies the given changes to the state. If there are outstanding
    /// `AnalysisSnapshot`s they will be canceled.
    pub fn apply_change(&mut self, change: AnalysisChange) {
        self.db.apply_change(change);
    }

    /// Creates a snapshot of the current `Analysis`. You can query the
    /// resulting `AnalysisSnapshot` to get analysis and diagnostics.
    pub fn snapshot(&self) -> AnalysisSnapshot {
        AnalysisSnapshot {
            db: self.db.snapshot(),
        }
    }

    /// Requests any outstanding snapshot to cancel computations.
    pub fn request_cancelation(&mut self) {
        self.db.request_cancelation();
    }
}

/// The `AnalysisSnapshot` is a snapshot of the state of the source, it enables
/// querying for the snapshot in a consistent state.
///
/// A `AnalysisSnapshot` is created by calling `Analysis::snapshot`. When
/// applying changes to the `Analysis` struct through the use of
/// `Analysis::apply_changes` all snapshots are cancelled (most methods return
/// `Err(Canceled)`).
pub struct AnalysisSnapshot {
    db: Snapshot<AnalysisDatabase>,
}

impl AnalysisSnapshot {
    /// Returns the syntax tree of the file.
    pub fn parse(&self, file_id: FileId) -> Cancelable<SourceFile> {
        self.with_db(|db| db.parse(file_id).tree())
    }

    /// Computes the set of diagnostics for the given file.
    pub fn diagnostics(&self, file_id: FileId) -> Cancelable<Vec<Diagnostic>> {
        self.with_db(|db| diagnostics::diagnostics(db, file_id))
    }

    /// Returns all the source files of the given package
    pub fn package_source_files(&self, package_id: PackageId) -> Cancelable<Vec<FileId>> {
        self.with_db(|db| {
            let packages = db.packages();
            let source_root = db.source_root(packages[package_id].source_root);
            source_root.files().collect()
        })
    }

    /// Returns the line index for the specified file
    pub fn file_line_index(&self, file_id: FileId) -> Cancelable<Arc<LineIndex>> {
        self.with_db(|db| db.line_index(file_id))
    }

    /// Returns a tree structure of the symbols of a file.
    pub fn file_structure(
        &self,
        file_id: FileId,
    ) -> Cancelable<Vec<file_structure::StructureNode>> {
        self.with_db(|db| file_structure::file_structure(&db.parse(file_id).tree()))
    }

    /// Computes completions at the given position
    pub fn completions(
        &self,
        position: FilePosition,
    ) -> Cancelable<Option<Vec<completion::CompletionItem>>> {
        self.with_db(|db| completion::completions(db, position).map(Into::into))
    }

    /// Performs an operation on that may be Canceled.
    fn with_db<F: FnOnce(&AnalysisDatabase) -> T + std::panic::UnwindSafe, T>(
        &self,
        f: F,
    ) -> Cancelable<T> {
        self.db.catch_canceled(f)
    }
}

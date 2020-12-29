use crate::db::AnalysisDatabase;
use hir::SourceDatabase;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// Represents an atomic change to the state of the `Analysis`
#[derive(Default)]
pub struct AnalysisChange {
    new_roots: Vec<(hir::SourceRootId, hir::PackageId)>,
    roots_changed: HashMap<hir::SourceRootId, RootChange>,
    files_changed: Vec<(hir::FileId, Arc<str>)>,
}

impl fmt::Debug for AnalysisChange {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut d = fmt.debug_struct("AnalysisChange");
        if !self.new_roots.is_empty() {
            d.field("new_roots", &self.new_roots);
        }
        if !self.roots_changed.is_empty() {
            d.field("roots_changed", &self.roots_changed);
        }
        if !self.files_changed.is_empty() {
            d.field("files_changed", &self.files_changed.len());
        }
        d.finish()
    }
}

impl AnalysisChange {
    /// Constructs a new `AnalysisChange`
    pub fn new() -> Self {
        AnalysisChange::default()
    }

    /// Records the addition of a new root
    pub fn add_root(&mut self, root_id: hir::SourceRootId, package_id: hir::PackageId) {
        self.new_roots.push((root_id, package_id));
    }

    /// Records the addition of a new file to a root
    pub fn add_file(
        &mut self,
        root_id: hir::SourceRootId,
        file_id: hir::FileId,
        path: hir::RelativePathBuf,
        text: Arc<str>,
    ) {
        let file = AddFile {
            file_id,
            path,
            text,
        };
        self.roots_changed
            .entry(root_id)
            .or_default()
            .added
            .push(file);
    }

    /// Records the change of content of a specific file
    pub fn change_file(&mut self, file_id: hir::FileId, new_text: Arc<str>) {
        self.files_changed.push((file_id, new_text))
    }

    /// Records the removal of a file from a root
    pub fn remove_file(
        &mut self,
        root_id: hir::SourceRootId,
        file_id: hir::FileId,
        path: hir::RelativePathBuf,
    ) {
        let file = RemoveFile { file_id, path };
        self.roots_changed
            .entry(root_id)
            .or_default()
            .removed
            .push(file);
    }
}

/// Represents the addition of a file to a source root.
#[derive(Debug)]
struct AddFile {
    file_id: hir::FileId,
    path: hir::RelativePathBuf,
    text: Arc<str>,
}

/// Represents the removal of a file from a source root.
#[derive(Debug)]
struct RemoveFile {
    file_id: hir::FileId,
    path: hir::RelativePathBuf,
}

/// Represents the changes to a source root.
#[derive(Default)]
struct RootChange {
    added: Vec<AddFile>,
    removed: Vec<RemoveFile>,
}

impl fmt::Debug for RootChange {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("RootChange")
            .field("added", &self.added.len())
            .field("removed", &self.removed.len())
            .finish()
    }
}

impl AnalysisDatabase {
    /// Applies the specified change to the database
    pub(crate) fn apply_change(&mut self, change: AnalysisChange) {
        // Add new source roots
        for (root_id, package_id) in change.new_roots {
            let root = hir::SourceRoot::new();
            self.set_source_root(root_id, Arc::new(root));
            self.set_package_source_root(package_id, root_id);
        }

        // Modify existing source roots
        for (root_id, root_change) in change.roots_changed {
            let mut source_root = hir::SourceRoot::clone(&self.source_root(root_id));
            for add_file in root_change.added {
                self.set_file_text(add_file.file_id, add_file.text);
                self.set_file_relative_path(add_file.file_id, add_file.path.clone());
                source_root.insert_file(add_file.file_id)
            }
            for remove_file in root_change.removed {
                self.set_file_text(remove_file.file_id, Arc::from(""));
                source_root.remove_file(remove_file.file_id);
            }
            self.set_source_root(root_id, Arc::new(source_root));
        }

        // Update changed files
        for (file_id, text) in change.files_changed {
            self.set_file_text(file_id, text)
        }
    }
}

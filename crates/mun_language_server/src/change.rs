use std::sync::Arc;

use mun_hir::SourceDatabase;

use crate::db::AnalysisDatabase;

/// Represents an atomic change to the state of the `Analysis`
#[derive(Default)]
pub struct AnalysisChange {
    packages: Option<mun_hir::PackageSet>,
    roots: Option<Vec<mun_hir::SourceRoot>>,
    files_changed: Vec<(mun_hir::FileId, Option<Arc<str>>)>,
}

impl AnalysisChange {
    /// Constructs a new `AnalysisChange`
    pub fn new() -> Self {
        AnalysisChange::default()
    }

    /// Sets the packages
    pub fn set_packages(&mut self, packages: mun_hir::PackageSet) {
        self.packages = Some(packages);
    }

    /// Records the addition of a new root
    pub fn set_roots(&mut self, roots: Vec<mun_hir::SourceRoot>) {
        self.roots = Some(roots);
    }

    /// Records the change of content of a specific file
    pub fn change_file(&mut self, file_id: mun_hir::FileId, new_text: Option<Arc<str>>) {
        self.files_changed.push((file_id, new_text));
    }
}

impl AnalysisDatabase {
    /// Applies the specified change to the database
    pub(crate) fn apply_change(&mut self, change: AnalysisChange) {
        // Add new package set
        if let Some(package_set) = change.packages {
            self.set_packages(Arc::new(package_set));
        }

        // Modify the source roots
        if let Some(roots) = change.roots {
            for (idx, root) in roots.into_iter().enumerate() {
                let root_id = mun_hir::SourceRootId(idx as u32);
                for file_id in root.files() {
                    self.set_file_source_root(file_id, root_id);
                }
                self.set_source_root(root_id, Arc::new(root));
            }
        }

        // Update changed files
        for (file_id, text) in change.files_changed {
            let text = text.unwrap_or_else(|| Arc::from("".to_owned()));
            self.set_file_text(file_id, text);
        }
    }
}

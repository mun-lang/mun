use std::sync::Arc;

use mun_paths::RelativePathBuf;

use crate::{FileId, LineIndex, ModuleTree, PackageId, PackageSet, SourceRoot, SourceRootId};

/// Database which stores all significant input facts: source code and project
/// model.
#[salsa::query_group(SourceDatabaseStorage)]
#[allow(clippy::trait_duplication_in_bounds)]
pub trait SourceDatabase: salsa::Database {
    /// Text of the file.
    #[salsa::input]
    fn file_text(&self, file_id: FileId) -> Arc<str>;

    /// Source root of a file
    #[salsa::input]
    fn file_source_root(&self, file_id: FileId) -> SourceRootId;

    /// Returns the set of packages
    #[salsa::input]
    fn packages(&self) -> Arc<PackageSet>;

    /// Contents of the source root
    #[salsa::input]
    fn source_root(&self, id: SourceRootId) -> Arc<SourceRoot>;

    /// Returns the relative path of a file
    fn file_relative_path(&self, file_id: FileId) -> RelativePathBuf;

    /// For a package, returns its hierarchy of modules.
    #[salsa::invoke(ModuleTree::module_tree_query)]
    fn module_tree(&self, package: PackageId) -> Arc<ModuleTree>;

    /// Returns the line index of a file
    #[salsa::invoke(line_index_query)]
    fn line_index(&self, file_id: FileId) -> Arc<LineIndex>;
}

/// Computes the relative path of a specific [`FileId`] within a [`SourceRoot`].
fn file_relative_path(db: &dyn SourceDatabase, file_id: FileId) -> RelativePathBuf {
    let source_root_id = db.file_source_root(file_id);
    let source_root = db.source_root(source_root_id);
    source_root.relative_path(file_id).to_relative_path_buf()
}
/// Computes a new `LineIndex` for the specified [`FileId`].
fn line_index_query(db: &dyn SourceDatabase, file_id: FileId) -> Arc<LineIndex> {
    let text = db.file_text(file_id);
    Arc::new(LineIndex::new(text.as_ref()))
}

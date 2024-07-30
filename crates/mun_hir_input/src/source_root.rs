use mun_paths::{RelativePath, RelativePathBuf};
use rustc_hash::FxHashMap;

use crate::FileId;

/// Files are grouped into [`SourceRoot`]. A source root is a directory on the
/// file systems which is watched for changes. Typically, it corresponds to a
/// single library.
///
/// Paths to files are always relative to a source root, the compiler does not
/// know the root path of the source root at all. So, a file from one source
/// root can't refer to a file in another source root by path.
///
/// Multiple source roots can be present if the language server is monitoring
/// multiple directories.
///
/// [`SourceRoot`]s are identified by a unique [`SourceRootId`].
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct SourceRoot {
    files: FxHashMap<FileId, RelativePathBuf>,
}

impl SourceRoot {
    pub fn insert_file(&mut self, file_id: FileId, path: impl AsRef<RelativePath>) {
        self.files
            .insert(file_id, path.as_ref().to_relative_path_buf());
    }
    pub fn remove_file(&mut self, file_id: FileId) -> bool {
        self.files.remove(&file_id).is_some()
    }
    pub fn relative_path(&self, file_id: FileId) -> &RelativePath {
        &self.files[&file_id]
    }
    pub fn files(&self) -> impl Iterator<Item = FileId> + '_ {
        self.files.keys().copied()
    }
}

/// A unique identifier of a [`SourceRoot`].
///
/// When referring to a [`SourceRoot`] it is preferable to use this identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceRootId(pub u32);

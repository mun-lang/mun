use paths::{RelativePath, RelativePathBuf};
use rustc_hash::FxHashMap;

/// `FileId` is an integer which uniquely identifies a file. File paths are messy and
/// system-dependent, so most of the code should work directly with `FileId`, without inspecting the
/// path. The mapping between `FileId` and path and `SourceRoot` is constant. A file rename is
/// represented as a pair of deletion/creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileId(pub u32);

/// Files are grouped into source roots. A source root is a directory on the file systems which is
/// watched for changes. Typically it corresponds to a single library.
///
/// Paths to files are always relative to a source root, the compiler does not know the root path
/// of the source root at all. So, a file from one source root can't refer to a file in another
/// source root by path.
///
/// Multiple source roots can be present if the language server is monitoring multiple directories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SourceRootId(pub u32);

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

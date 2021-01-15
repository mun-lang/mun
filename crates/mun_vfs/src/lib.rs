use std::mem;

pub use monitor::{
    Monitor, MonitorConfig, MonitorDirectories, MonitorEntry, MonitorMessage, NotifyMonitor,
};

use path_interner::PathInterner;
use paths::{AbsPath, AbsPathBuf};

mod monitor;
mod path_interner;

/// A `FileId` represents a unique identifier for a file within the `VirtualFileSystem`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct FileId(pub u32);

/// The `VirtualFileSystem` is a struct that manages a set of files and their content. Changes to
/// the instance are logged, they can be be retrieved via the `take_changes` method.
#[derive(Default)]
pub struct VirtualFileSystem {
    /// Used to convert from paths to `FileId` and vice versa.
    interner: PathInterner,

    /// Per file the content of the file, or `None` if no content is available
    file_contents: Vec<Option<Vec<u8>>>,

    /// A record of changes to this instance.
    changes: Vec<ChangedFile>,
}

/// A record of a change to a file
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChangedFile {
    pub file_id: FileId,
    pub kind: ChangeKind,
}

impl ChangedFile {
    /// Returns true if this change indicates that the file was created or deleted
    pub fn is_created_or_deleted(&self) -> bool {
        matches!(self.kind, ChangeKind::Create | ChangeKind::Delete)
    }
}

/// The type of change that a file undergoes
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChangeKind {
    Create,
    Modify,
    Delete,
}

impl VirtualFileSystem {
    /// Returns `true` if there are changes that can be processed.
    pub fn has_changes(&self) -> bool {
        !self.changes.is_empty()
    }

    /// Returns the changes performed on the instance since the last time this function was called
    /// or since the creation of the instance.
    pub fn take_changes(&mut self) -> Vec<ChangedFile> {
        mem::take(&mut self.changes)
    }

    /// Returns the `FileId` of the file at the specified `path` or `None` if there is no data for
    /// that file.
    pub fn file_id(&self, path: &AbsPath) -> Option<FileId> {
        self.interner
            .get(path)
            .filter(|&file_id| self.get(file_id).is_some())
    }

    /// Returns the path of the file with the specified `FileId`.
    pub fn file_path(&self, file_id: FileId) -> &AbsPath {
        self.interner.lookup(file_id)
    }

    /// Returns the content of the file with the specified `FileId`.
    pub fn file_contents(&self, file_id: FileId) -> Option<&[u8]> {
        self.get(file_id).as_deref()
    }

    /// Returns an iterator that iterates all `FileId`s and their path.
    pub fn iter(&self) -> impl Iterator<Item = (FileId, &AbsPath)> + '_ {
        self.file_contents
            .iter()
            .enumerate()
            .filter(|(_, contents)| contents.is_some())
            .map(move |(id, _)| {
                let file_id = FileId(id as u32);
                let path = self.interner.lookup(file_id);
                (file_id, path)
            })
    }

    /// Notifies this instance that the contents of the specified file has changed to something
    /// else. Returns true if the new contents is actually different.
    pub fn set_file_contents(&mut self, path: &AbsPath, contents: Option<Vec<u8>>) -> bool {
        let file_id = self.alloc_file_id(path);
        let kind = match (&self.get(file_id), &contents) {
            (None, None) => return false,
            (None, Some(_)) => ChangeKind::Create,
            (Some(_), None) => ChangeKind::Delete,
            (Some(old), Some(new)) if old == new => return false,
            (Some(_), Some(_)) => ChangeKind::Modify,
        };

        *self.get_mut(file_id) = contents;
        self.changes.push(ChangedFile { file_id, kind });
        true
    }

    /// Returns the `FileId` for the specified path and ensures that we can use it with this
    /// instance.
    fn alloc_file_id(&mut self, path: &AbsPath) -> FileId {
        let file_id = self.interner.intern(path);
        let idx = file_id.0 as usize;
        let len = self.file_contents.len().max(idx + 1);
        self.file_contents.resize(len, None);
        file_id
    }

    /// Returns a reference to the current content of a specific file. This function is only used
    /// internally. Use the `file_contents` function to get the contents of a file.
    fn get(&self, file_id: FileId) -> &Option<Vec<u8>> {
        &self.file_contents[file_id.0 as usize]
    }

    /// Returns a mutable reference to the current content of a specific file. This function is only
    /// used internally. Use the `set_file_contents` function to update the contents of a file.
    fn get_mut(&mut self, file_id: FileId) -> &mut Option<Vec<u8>> {
        &mut self.file_contents[file_id.0 as usize]
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;
    use std::path::PathBuf;

    use crate::{AbsPathBuf, ChangeKind, ChangedFile, VirtualFileSystem};

    #[test]
    fn vfs() {
        let mut vfs = VirtualFileSystem::default();
        assert!(!vfs.has_changes());

        // Construct a fake file name
        let abs_manifest_dir: AbsPathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .try_into()
            .unwrap();
        let test_path = abs_manifest_dir.as_path().join("test");

        // We should not have a FileId for this file yet
        assert!(vfs.file_id(&test_path).is_none());

        // Store some data in the vfs, this should definitly trigger a change
        assert!(vfs.set_file_contents(&test_path, Some(vec![])), true);
        assert!(vfs.has_changes());

        // We should now have a FileId
        let file_id = vfs
            .file_id(&test_path)
            .expect("there should be a FileId by now");

        // Lookup the path, it should match
        assert_eq!(&test_path, vfs.file_path(file_id));

        // Get the contents of the file
        assert!(vfs.file_contents(file_id).is_some());

        // Modify the file contents, but dont actually modify it, should not trigger a change
        assert_eq!(vfs.set_file_contents(&test_path, Some(vec![])), false);

        // Actually modify the contents
        assert!(vfs.set_file_contents(&test_path, Some(vec![0])), true);

        // Remove the file contents, should also trigger a change
        assert!(vfs.set_file_contents(&test_path, None), true);

        // We should now no longer have a file id because the contents was removed
        assert_eq!(vfs.file_id(&test_path), None);

        // Get the changes
        assert!(vfs.has_changes());
        assert_eq!(
            vfs.take_changes(),
            vec![
                ChangedFile {
                    file_id,
                    kind: ChangeKind::Create
                },
                ChangedFile {
                    file_id,
                    kind: ChangeKind::Modify
                },
                ChangedFile {
                    file_id,
                    kind: ChangeKind::Delete
                },
            ]
        );
    }

    #[test]
    fn iter() {
        let mut vfs = VirtualFileSystem::default();

        // Construct a fake file name
        let abs_manifest_dir: AbsPathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .try_into()
            .unwrap();

        // Add two files to the system
        let test_path2 = abs_manifest_dir.as_path().join("test2");
        let test_path = abs_manifest_dir.as_path().join("test");
        assert!(vfs.set_file_contents(&test_path, Some(vec![0])));
        assert!(vfs.set_file_contents(&test_path2, Some(vec![1])));
        let file_id = vfs.file_id(&test_path).unwrap();
        let file_id2 = vfs.file_id(&test_path2).unwrap();
        assert_ne!(file_id, file_id2);

        let mut entries = vfs
            .iter()
            .map(|(id, entry)| (id, entry.to_path_buf()))
            .collect::<Vec<_>>();
        let mut expected_entries =
            vec![(file_id, test_path.clone()), (file_id2, test_path2.clone())];

        entries.sort_by_key(|entry| entry.0);
        expected_entries.sort_by_key(|entry| entry.0);

        assert_eq!(entries, expected_entries);
    }
}

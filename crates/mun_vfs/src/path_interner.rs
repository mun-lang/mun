use crate::{AbsPath, AbsPathBuf, FileId};
use rustc_hash::FxHashMap;

/// A struct to map file paths to `FileId`s. `FileId`s are never cleared because we assume there
/// never be too many.
#[derive(Default)]
pub(crate) struct PathInterner {
    path_to_id: FxHashMap<AbsPathBuf, FileId>,
    id_to_path: Vec<AbsPathBuf>,
}

impl PathInterner {
    /// Returns the `FileId` for the specified `path` or `None` if the specified path was not
    /// interned.
    pub fn get(&self, path: &AbsPath) -> Option<FileId> {
        self.path_to_id.get(path).copied()
    }

    /// Interns the specified `path`, returning a unique `FileId` for the path.
    pub fn intern(&mut self, path: &AbsPath) -> FileId {
        if let Some(id) = self.get(path) {
            id
        } else {
            let id = FileId(self.id_to_path.len() as u32);
            self.path_to_id.insert(path.to_path_buf(), id);
            self.id_to_path.push(path.to_path_buf());
            id
        }
    }

    /// Returns the path for the specified `FileId`.
    pub fn lookup(&self, id: FileId) -> &AbsPath {
        &self.id_to_path[id.0 as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::PathInterner;
    use crate::AbsPathBuf;
    use std::convert::TryInto;
    use std::path::PathBuf;

    #[test]
    fn intern() {
        let mut interner = PathInterner::default();

        let file_path_buf: PathBuf = env!("CARGO_MANIFEST_DIR").into();
        let abs_file: AbsPathBuf = file_path_buf.try_into().unwrap();

        // Didnt intern yet, should not be able to find file_id
        assert_eq!(interner.get(&abs_file), None);

        // Insert the path into the interner
        let file_id = interner.intern(&abs_file);

        // We get get the file_id by path now
        assert_eq!(interner.get(&abs_file), Some(file_id));

        // Insert the path again, should return the same path
        let file_id2 = interner.intern(&abs_file);
        assert_eq!(file_id, file_id2);

        // Check the path from the id
        assert_eq!(&abs_file, interner.lookup(file_id));
    }
}

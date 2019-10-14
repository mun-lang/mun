use super::Library;
use std::path::Path;
use failure::Error;
use std::fs;

/// A structure that holds a `Library` instance but creates a unique file per load this enables
/// writing to the original library and ensures that each shared object on Linux is loaded
/// separately.
///
/// There is no risk of cleaning the temporary file while it is used because loading the library
/// keeps the file open (Windows) or keeping the file is not required in the first place (*nix).
pub struct TempLibrary {
    tmp_path: tempfile::TempPath,
    library: Library
}

impl TempLibrary {
    pub fn new(path: &Path) -> Result<Self, Error> {
        let tmp_path = tempfile::NamedTempFile::new()?.into_temp_path();
        fs::copy(path, &tmp_path);
        let library = Library::new(&tmp_path)?;
        Ok(TempLibrary {
            tmp_path,
            library
        })
    }

    /// Returns the loaded library
    pub fn library(&self) -> &Library {
        &self.library
    }
}

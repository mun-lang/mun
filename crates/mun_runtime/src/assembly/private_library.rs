use std::path::Path;

use failure::Error;
use libloading::Library;

/// A structure that holds a `Library` instance but enables writing to the original library.
///
/// On Windows while a library is loaded it cannot be written to. On Windows this library copies the
/// library to a temporary path and loads the library from there. When the `PrivateLibrary` is
/// dropped, the internal library is unloaded and the temporary file is deleted.
///
/// There is no risk of Windows cleaning the temporary file while it is used because loading the
/// library keeps the file open.
pub struct PrivateLibrary {
    #[cfg(target_os = "windows")]
    tmp_path: tempfile::TempPath,
    library: Library,
}

impl PrivateLibrary {
    #[cfg(not(target_os = "windows"))]
    pub fn new(path: &Path) -> Result<Self, Error> {
        let library = Library::new(path)?;
        Ok(PrivateLibrary { library })
    }

    #[cfg(target_os = "windows")]
    pub fn new(path: &Path) -> Result<Self, Error> {
        let tmp_path = tempfile::NamedTempFile::new()?.into_temp_path();
        let library = Library::new(&tmp_path)?;
        Ok(PrivateLibrary { tmp_path, library })
    }

    /// Returns the loaded library
    pub fn library(&self) -> &Library {
        &self.library
    }
}

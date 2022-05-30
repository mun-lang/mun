use std::fs;
use std::path::Path;

use anyhow::Error;
use libloading::Library;

/// A structure that holds a `Library` instance but creates a unique file per load. This enables
/// writing to the original library and ensures that each shared object on Linux is loaded
/// separately.
///
/// There is no risk of cleaning the temporary file while it is used because loading the library
/// keeps the file open (Windows) or keeping the file is not required in the first place (*nix).
pub struct TempLibrary {
    _tmp_path: tempfile::TempPath,
    library: Library,
}

impl TempLibrary {
    /// Find and load a dynamic library.
    ///
    /// The `filename` argument may be either:
    ///
    /// * A library filename;
    /// * The absolute path to the library;
    /// * A relative (to the current working directory) path to the library.
    ///
    /// # Safety
    ///
    /// When a library is loaded, initialisation routines contained within it are executed.
    /// For the purposes of safety, the execution of these routines is conceptually the same calling
    /// an unknown foreign function and may impose arbitrary requirements on the caller for the call
    /// to be sound.
    ///
    /// Additionally, the callers of this function must also ensure that execution of the
    /// termination routines contained within the library is safe as well. These routines may be
    /// executed when the library is unloaded.
    ///
    /// See [`libloading::Library::new`] for more information.
    pub unsafe fn new(path: &Path) -> Result<Self, Error> {
        let tmp_path = tempfile::NamedTempFile::new()?.into_temp_path();
        fs::copy(path, &tmp_path)?;
        let library = Library::new(&tmp_path)?;
        Ok(TempLibrary {
            _tmp_path: tmp_path,
            library,
        })
    }

    /// Returns the loaded library
    pub fn library(&self) -> &Library {
        &self.library
    }
}

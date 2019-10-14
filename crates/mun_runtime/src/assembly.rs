use std::path::{Path, PathBuf};

use failure::Error;
use mun_abi::FunctionInfo;

mod library;
mod temp_library;

use self::library::Library;
use self::temp_library::TempLibrary;

/// An assembly is the smallest compilable unit of code in Mun.
pub struct Assembly {
    library_path: PathBuf,
    library: Option<TempLibrary>,
}

impl Assembly {
    /// Loads an assembly for the library at `library_path` and its dependencies.
    pub fn load(library_path: &Path) -> Result<Self, Error> {
        let library = TempLibrary::new(library_path)?;
        println!("Loaded module '{}'.", library_path.to_string_lossy());

        Ok(Assembly {
            library_path: library_path.to_path_buf(),
            library: Some(library),
        })
    }

    pub fn swap(&mut self, library_path: &Path) -> Result<(), Error> {
        let mut library = Some(TempLibrary::new(library_path)?);
        println!("Reloaded module '{}'.", library_path.to_string_lossy());

        std::mem::swap(&mut library, &mut self.library);
        self.library_path = library_path.to_path_buf();

        Ok(())
    }

    /// Retrieves the assembly's loaded shared library.
    pub fn library(&self) -> &Library {
        self.library.as_ref().expect("Library was not loaded.").library()
    }

    /// Retrieves all of the assembly's functions.
    pub fn functions(&self) -> impl Iterator<Item = &FunctionInfo> {
        self.library().module_info().functions().iter()
    }

    /// Returns the path corresponding tot the assembly's library.
    pub fn library_path(&self) -> &Path {
        self.library_path.as_path()
    }
}

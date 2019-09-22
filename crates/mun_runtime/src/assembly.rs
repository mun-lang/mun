use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::error::*;
use crate::library::Library;
use mun_abi::FunctionInfo;

const LIB_DIR: &str = "tmp";

/// An assembly is the smallest compilable unit of code in Mun.
pub struct Assembly {
    library_path: PathBuf,
    tmp_path: PathBuf,
    library: Option<Library>,
}

impl Assembly {
    /// Loads an assembly for the library at `library_path` and its dependencies.
    pub fn load(library_path: &Path) -> Result<Self> {
        let library_name = library_path.file_name().ok_or(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Incorrect library path.",
        ))?;

        let tmp_dir = env::current_dir()?.join(LIB_DIR);
        if !tmp_dir.exists() {
            fs::create_dir(&tmp_dir)?;
        }

        let tmp_path = tmp_dir.join(library_name);
        fs::copy(&library_path, &tmp_path)?;

        let library = Library::new(tmp_path.as_path())?;
        println!("Loaded module '{}'.", library_path.to_string_lossy());

        Ok(Assembly {
            library_path: library_path.to_path_buf(),
            tmp_path,
            library: Some(library),
        })
    }

    pub fn swap(&mut self, library_path: &Path) -> Result<()> {
        let library_path = library_path.canonicalize()?;
        let library_name = library_path.file_name().ok_or(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Incorrect library path.",
        ))?;

        let tmp_path = env::current_dir()?.join(LIB_DIR).join(library_name);

        // Drop the old library, as some operating systems don't allow editing of in-use shared libraries
        self.library.take();

        fs::copy(&library_path, &tmp_path)?;

        let library = Library::new(tmp_path.as_path())?;
        println!("Reloaded module '{}'.", library_path.to_string_lossy());

        self.library = Some(library);
        self.library_path = library_path;
        self.tmp_path = tmp_path;

        Ok(())
    }

    /// Retrieves the assembly's loaded shared library.
    pub fn library(&self) -> &Library {
        self.library.as_ref().expect("Library was not loaded.")
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

use std::io;
use std::path::{Path, PathBuf};

use crate::error::*;
use crate::library::Library;

/// A module is the smallest compilable unit of code in Mun.
pub struct Module {
    library: Option<Library>,
    manifest_path: PathBuf,
}

impl Module {
    /// Constructs a module for the manifest at `manifest_path`.
    pub fn new(manifest_path: &Path) -> io::Result<Module> {
        let manifest_path = manifest_path.canonicalize()?;

        Ok(Module {
            library: None,
            manifest_path,
        })
    }

    /// Retrieves the module's loaded shared library.
    pub fn library(&self) -> &Library {
        self.library.as_ref().expect("Library was not loaded.")
    }

    /// Returns the module's manifest path.
    pub fn manifest_path(&self) -> &Path {
        self.manifest_path.as_path()
    }

    /// Retrieves the module's shared library's filename.
    pub fn filename(&self) -> Result<String> {
        let config = cargo::Config::default()?;
        let workspace = cargo::core::Workspace::new(self.manifest_path(), &config)?;

        workspace
            .default_members()
            .next()
            .ok_or(
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Module workspace does not contain an output package.",
                )
                .into(),
            )
            .map(|package| package.name().to_string())
    }

    /// Compiles the module's code into a shared library.
    pub fn compile(&self) -> Result<PathBuf> {
        let config = cargo::Config::default()?;
        let workspace = cargo::core::Workspace::new(self.manifest_path(), &config)?;

        let compile_options =
            cargo::ops::CompileOptions::new(&config, cargo::core::compiler::CompileMode::Build)?;
        let compilation = cargo::ops::compile(&workspace, &compile_options)?;

        Ok(compilation.root_output.clone())
    }

    /// Loads the shared library at `output_dir` into the module.
    pub fn load(&mut self, output_dir: &Path) -> Result<()> {
        let mut lib_path = output_dir.to_path_buf();
        lib_path.push(self.filename()?);

        let library = Library::new(lib_path.as_path())?;
        self.library = Some(library);
        println!("Loaded module '{}'.", lib_path.to_string_lossy());
        Ok(())
    }

    /// Unloads the module's shared library.
    pub fn unload(&mut self) {
        self.library = None;
    }
}

use std::io;
use std::path::{Path, PathBuf};

use crate::error::*;
use crate::library::Library;

pub struct Module {
    library: Option<Library>,
    manifest_path: PathBuf,
}

impl Module {
    pub fn new(manifest_path: &Path) -> io::Result<Module> {
        let manifest_path = manifest_path.canonicalize()?;

        Ok(Module {
            library: None,
            manifest_path,
        })
    }

    pub fn library(&self) -> &Option<Library> {
        &self.library
    }

    pub fn manifest_path(&self) -> &Path {
        self.manifest_path.as_path()
    }

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

    pub fn compile(&self) -> Result<PathBuf> {
        let config = cargo::Config::default()?;
        let workspace = cargo::core::Workspace::new(self.manifest_path(), &config)?;

        let compile_options =
            cargo::ops::CompileOptions::new(&config, cargo::core::compiler::CompileMode::Build)?;
        let compilation = cargo::ops::compile(&workspace, &compile_options)?;

        Ok(compilation.root_output.clone())
    }

    pub fn load(&mut self, output_dir: &Path) -> Result<()> {
        let mut lib_path = output_dir.to_path_buf();
        lib_path.push(self.filename()?);

        let library = Library::new(lib_path.as_path())?;
        self.library = Some(library);
        println!("Loaded module '{}'.", lib_path.to_string_lossy());
        Ok(())
    }

    pub fn unload(&mut self) {
        self.library = None;
    }
}

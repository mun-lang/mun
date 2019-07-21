use std::io;
use std::path::{Path, PathBuf};

use crate::error::*;
use crate::library::Library;

pub struct Module {
    library: Option<Library>,
    src: PathBuf,
    dst: PathBuf,
}

impl Module {
    pub fn new(src: &Path, dst: &Path) -> io::Result<Module> {
        let src = src.canonicalize()?;
        let dst = dst.canonicalize()?;

        Ok(Module {
            library: None,
            src,
            dst,
        })
    }

    pub fn library(&self) -> &Option<Library> {
        &self.library
    }

    pub fn src(&self) -> &Path {
        self.src.as_path()
    }

    pub fn dst(&self) -> &Path {
        self.dst.as_path()
    }

    pub fn load(&mut self) -> Result<()> {
        let library = Library::new(self.dst.as_path())?;
        self.library = Some(library);
        println!("Loaded module '{}'.", self.dst.to_string_lossy());
        Ok(())
    }

    pub fn unload(&mut self) {
        self.library = None;
    }
}

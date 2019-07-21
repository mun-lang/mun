use std::path::Path;

use crate::error::*;
use libloading;

pub use libloading::Symbol;

pub struct Library {
    inner: libloading::Library,
}

impl Library {
    pub fn new(path: &Path) -> Result<Library> {
        let library = libloading::Library::new(path)?;
        Ok(Library { inner: library })
    }

    pub fn get_fn<T>(&self, name: &str) -> Result<Symbol<T>> {
        unsafe { self.inner.get(name.as_ref()) }.map_err(|e| e.into())
    }
}

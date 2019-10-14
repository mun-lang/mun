use std::path::Path;

use failure::Error;
use libloading::{self, Symbol};
use mun_abi::ModuleInfo;

/// A wrapper for a shared library and its corresponding symbol metadata.
pub struct Library {
    inner: libloading::Library,
    symbols: ModuleInfo,
}

impl Library {
    /// Loads the shared library at `path`, retrieves its symbol metadata, and constructs a library
    /// wrapper.
    pub fn new(path: &Path) -> Result<Library, Error> {
        let library = libloading::Library::new(path)?;

        // Check whether the library has a symbols function
        let get_symbols: Symbol<'_, extern "C" fn() -> ModuleInfo> =
            unsafe { library.get(b"get_symbols") }?;

        let symbols = get_symbols();

        Ok(Library {
            inner: library,
            symbols,
        })
    }

    /// Retrieves the inner shared library.
    pub fn inner(&self) -> &libloading::Library {
        &self.inner
    }

    /// Retrieves the libraries symbol metadata.
    pub fn module_info(&self) -> &ModuleInfo {
        &self.symbols
    }
}

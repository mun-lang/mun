use std::io;
use std::path::{Path, PathBuf};

use crate::DispatchTable;
use failure::Error;
use libloading::Symbol;
use mun_abi::AssemblyInfo;

mod private_library;

use self::private_library::PrivateLibrary;

/// An assembly is the smallest compilable unit of code in Mun.
pub struct Assembly {
    library_path: PathBuf,
    library: Option<PrivateLibrary>,
    info: AssemblyInfo,
}

impl Assembly {
    /// Loads an assembly for the library at `library_path` and its dependencies.
    pub fn load(
        library_path: &Path,
        runtime_dispatch_table: &mut DispatchTable,
    ) -> Result<Self, Error> {
        let library = PrivateLibrary::new(library_path)?;

        // Check whether the library has a symbols function
        let get_info: Symbol<'_, extern "C" fn() -> AssemblyInfo> =
            unsafe { library.library().get(b"get_info") }?;

        let info = get_info();

        for function in info.symbols.functions() {
            runtime_dispatch_table.insert(function.signature.name(), function.clone());
        }

        Ok(Assembly {
            library_path: library_path.to_path_buf(),
            library: Some(library),
            info,
        })
    }

    pub fn link(&mut self, runtime_dispatch_table: &DispatchTable) -> Result<(), Error> {
        for (dispatch_ptr, fn_signature) in self.info.dispatch_table.iter_mut() {
            let fn_ptr = runtime_dispatch_table
                .get(fn_signature.name())
                .map(|f| f.fn_ptr)
                .ok_or(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "Failed to link: function '{}' is missing.",
                        fn_signature.name()
                    ),
                ))?;

            *dispatch_ptr = fn_ptr;
        }
        Ok(())
    }

    pub fn swap(
        &mut self,
        library_path: &Path,
        runtime_dispatch_table: &mut DispatchTable,
    ) -> Result<(), Error> {
        // let library_path = library_path.canonicalize()?;

        // Drop the old library, as some operating systems don't allow editing of in-use shared libraries
        self.library.take();

        for function in self.info.symbols.functions() {
            runtime_dispatch_table.remove(function.signature.name());
        }

        // TODO: Partial hot reload of an assembly
        *self = Assembly::load(library_path, runtime_dispatch_table)?;
        Ok(())
    }

    /// Retrieves the assembly's loaded shared library.
    pub fn info(&self) -> &AssemblyInfo {
        &self.info
    }

    /// Returns the path corresponding tot the assembly's library.
    pub fn library_path(&self) -> &Path {
        self.library_path.as_path()
    }
}

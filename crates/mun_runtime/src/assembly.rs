use std::io;
use std::path::{Path, PathBuf};

use crate::{Allocator, DispatchTable};
use abi::AssemblyInfo;
use failure::Error;
use libloading::Symbol;

mod temp_library;

use self::temp_library::TempLibrary;
use std::sync::Arc;

/// An assembly is a hot reloadable compilation unit, consisting of one or more Mun modules.
pub struct Assembly {
    library_path: PathBuf,
    library: Option<TempLibrary>,
    info: AssemblyInfo,
    allocator: Arc<Allocator>,
}

impl Assembly {
    /// Loads an assembly and its information for the shared library at `library_path`.
    pub fn load(
        library_path: &Path,
        runtime_dispatch_table: &mut DispatchTable,
        allocator: Arc<Allocator>,
    ) -> Result<Self, Error> {
        let library = TempLibrary::new(library_path)?;

        // Check whether the library has a symbols function
        let get_info: Symbol<'_, extern "C" fn() -> AssemblyInfo> =
            unsafe { library.library().get(b"get_info") }?;

        let set_allocator_handle: Symbol<'_, extern "C" fn(*mut std::ffi::c_void)> =
            unsafe { library.library().get(b"set_allocator_handle") }?;

        let allocator_ptr = Arc::into_raw(allocator.clone()) as *mut std::ffi::c_void;
        set_allocator_handle(allocator_ptr);

        let info = get_info();

        for function in info.symbols.functions() {
            runtime_dispatch_table.insert_fn(function.signature.name(), function.clone());
        }

        Ok(Assembly {
            library_path: library_path.to_path_buf(),
            library: Some(library),
            info,
            allocator,
        })
    }

    /// Links the assembly using the runtime's dispatch table.
    pub fn link(&mut self, runtime_dispatch_table: &DispatchTable) -> Result<(), Error> {
        for (dispatch_ptr, fn_signature) in self.info.dispatch_table.iter_mut() {
            let fn_ptr = runtime_dispatch_table
                .get_fn(fn_signature.name())
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("Failed to link: function '{}' is missing.", fn_signature),
                    )
                })
                .and_then(|dispatch_func| {
                    // TODO: This is a hack
                    if dispatch_func.signature.return_type() != fn_signature.return_type() ||
                        dispatch_func.signature.arg_types().len() != fn_signature.arg_types().len() ||
                        !dispatch_func.signature.arg_types().iter().zip(fn_signature.arg_types().iter()).all(|(a,b)| PartialEq::eq(a,b)) {
                        Err(io::Error::new(
                            io::ErrorKind::NotFound,
                            format!("Failed to link while looking for function '{}', a function by the same name does exists but the signatures do not match ({}).", fn_signature, dispatch_func.signature),
                        ))
                    } else {
                        Ok(dispatch_func)
                    }
                })
                .map(|f| f.fn_ptr)?;

            *dispatch_ptr = fn_ptr;
        }
        Ok(())
    }

    /// Swaps the assembly's shared library and its information for the library at `library_path`.
    pub fn swap(
        &mut self,
        library_path: &Path,
        runtime_dispatch_table: &mut DispatchTable,
    ) -> Result<(), Error> {
        // let library_path = library_path.canonicalize()?;

        for function in self.info.symbols.functions() {
            runtime_dispatch_table.remove_fn(function.signature.name());
        }

        // Drop the old library, as some operating systems don't allow editing of in-use shared
        // libraries
        self.library.take();

        // TODO: Partial hot reload of an assembly
        *self = Assembly::load(library_path, runtime_dispatch_table, self.allocator.clone())?;
        Ok(())
    }

    /// Returns the assembly's information.
    pub fn info(&self) -> &AssemblyInfo {
        &self.info
    }

    /// Returns the path corresponding to the assembly's library.
    pub fn library_path(&self) -> &Path {
        self.library_path.as_path()
    }
}

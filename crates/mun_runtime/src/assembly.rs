use std::io;
use std::path::{Path, PathBuf};

use crate::DispatchTable;
use abi::AssemblyInfo;
use libloading::Symbol;

mod temp_library;

use self::temp_library::TempLibrary;
use crate::garbage_collector::{GarbageCollector, UnsafeTypeInfo};
use memory::mapping::{Mapping, MemoryMapper};
use std::{collections::HashSet, ptr::NonNull, sync::Arc};

/// An assembly is a hot reloadable compilation unit, consisting of one or more Mun modules.
pub struct Assembly {
    library_path: PathBuf,
    library: TempLibrary,
    legacy_libs: Vec<TempLibrary>,
    info: AssemblyInfo,
    allocator: Arc<GarbageCollector>,
}

impl Assembly {
    /// Loads an assembly and its information for the shared library at `library_path`. The
    /// resulting `Assembly` is ensured to be linkable.
    pub fn load(
        library_path: &Path,
        gc: Arc<GarbageCollector>,
        runtime_dispatch_table: &DispatchTable,
    ) -> Result<Self, failure::Error> {
        let library = TempLibrary::new(library_path)?;

        // Check whether the library has a symbols function
        let get_info: Symbol<'_, extern "C" fn() -> AssemblyInfo> =
            unsafe { library.library().get(b"get_info") }?;

        let set_allocator_handle: Symbol<'_, extern "C" fn(*mut std::ffi::c_void)> =
            unsafe { library.library().get(b"set_allocator_handle") }?;

        let allocator_ptr = Arc::into_raw(gc.clone()) as *mut std::ffi::c_void;
        set_allocator_handle(allocator_ptr);

        let info = get_info();
        let assembly = Assembly {
            library_path: library_path.to_path_buf(),
            library,
            legacy_libs: Vec::new(),
            info,
            allocator: gc,
        };

        // Ensure that any loaded `Assembly` can be linked safely.
        assembly.ensure_linkable(runtime_dispatch_table)?;
        Ok(assembly)
    }

    /// Verifies that the `Assembly` resolves all dependencies in the `DispatchTable`.
    fn ensure_linkable(&self, runtime_dispatch_table: &DispatchTable) -> Result<(), io::Error> {
        let fn_names: HashSet<&str> = self
            .info
            .symbols
            .functions()
            .iter()
            .map(|f| f.prototype.name())
            .collect();

        for (fn_ptr, fn_prototype) in self.info.dispatch_table.iter() {
            // Only take signatures into account that do *not* yet have a function pointer assigned
            // by the compiler.
            if !fn_ptr.is_null() {
                continue;
            }

            // Ensure that the required function is in the runtime dispatch table and that its signature
            // is the same.
            match runtime_dispatch_table.get_fn(fn_prototype.name()) {
                Some(fn_definition) => {
                    if fn_prototype.signature != fn_definition.prototype.signature {
                        return Err(io::Error::new(
                            io::ErrorKind::NotFound,
                            format!("Failed to link: function '{}' is missing. A function with the same name does exist, but the signatures do not match (expected: {}, found: {}).", fn_prototype.name(), fn_prototype, fn_definition.prototype),
                        ));
                    }
                }
                None => {
                    return Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        format!(
                            "Failed to link: function `{}` is missing.",
                            fn_prototype.name()
                        ),
                    ))
                }
            }
        }

        if let Some(dependencies) = runtime_dispatch_table
            .fn_dependencies
            .get(self.info.symbols.path())
        {
            for fn_name in dependencies.keys() {
                if !fn_names.contains(&fn_name.as_str()) {
                    return Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("Failed to link: function `{}` is missing.", fn_name),
                    ));
                }
            }

            for fn_definition in self.info.symbols.functions().iter() {
                let (fn_prototype, _) = dependencies
                    .get(fn_definition.prototype.name())
                    .expect("The dependency must exist after the previous check.");

                if fn_prototype.signature != fn_definition.prototype.signature {
                    return Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("Failed to link: function '{}' is missing. A function with the same name does exist, but the signatures do not match (expected: {}, found: {}).", fn_prototype.name(), fn_prototype, fn_definition.prototype),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Links the assembly using the runtime's dispatch table.
    ///
    /// Requires that `ensure_linkable` has been called beforehand. This happens upon creation of
    /// an `Assembly` - in the `load` function - making this function safe.
    pub fn link(&mut self, runtime_dispatch_table: &mut DispatchTable) {
        for function in self.info.symbols.functions() {
            runtime_dispatch_table.insert_fn(function.prototype.name(), function.clone());
        }

        for (dispatch_ptr, fn_prototype) in self.info.dispatch_table.iter_mut() {
            if dispatch_ptr.is_null() {
                let fn_ptr = runtime_dispatch_table
                    .get_fn(fn_prototype.name())
                    .unwrap_or_else(|| panic!("Function '{}' is expected to exist.", fn_prototype))
                    .fn_ptr;
                *dispatch_ptr = fn_ptr;
            }
        }
    }

    /// Swaps the assembly's shared library and its information for the library at `library_path`.
    pub fn swap(
        &mut self,
        library_path: &Path,
        runtime_dispatch_table: &mut DispatchTable,
    ) -> Result<(), failure::Error> {
        let mut new_assembly =
            Assembly::load(library_path, self.allocator.clone(), runtime_dispatch_table)?;

        let old_types: Vec<UnsafeTypeInfo> = self
            .info
            .symbols
            .types()
            .iter()
            .map(|ty| {
                // Safety: `ty` is a shared reference, so is guaranteed to not be `ptr::null()`.
                UnsafeTypeInfo::new(unsafe {
                    NonNull::new_unchecked(*ty as *const abi::TypeInfo as *mut _)
                })
            })
            .collect();

        let new_types: Vec<UnsafeTypeInfo> = new_assembly
            .info
            .symbols
            .types()
            .iter()
            .map(|ty| {
                // Safety: `ty` is a shared reference, so is guaranteed to not be `ptr::null()`.
                UnsafeTypeInfo::new(unsafe {
                    NonNull::new_unchecked(*ty as *const abi::TypeInfo as *mut _)
                })
            })
            .collect();

        let mapping = Mapping::new(&old_types, &new_types);
        let deleted_objects = self.allocator.map_memory(mapping);

        // Remove the old assembly's functions
        for function in self.info.symbols.functions() {
            runtime_dispatch_table.remove_fn(function.prototype.name());
        }

        new_assembly.link(runtime_dispatch_table);

        // Retain all existing legacy libs
        new_assembly.legacy_libs.append(&mut self.legacy_libs);

        std::mem::swap(self, &mut new_assembly);
        let old_assembly = new_assembly;

        if !deleted_objects.is_empty() {
            // Retain the previous assembly
            self.legacy_libs.push(old_assembly.into_library());
        }

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

    /// Converts the `Assembly` into a `TempLibrary`, consuming the input in the process.
    pub fn into_library(self) -> TempLibrary {
        self.library
    }
}

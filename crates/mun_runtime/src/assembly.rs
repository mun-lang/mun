use crate::{garbage_collector::GarbageCollector, DispatchTable};
use abi::{AssemblyInfo, FunctionPrototype};
use anyhow::anyhow;
use libloader::{MunLibrary, TempLibrary};
use log::error;
use memory::{
    mapping::{Mapping, MemoryMapper},
    type_table::TypeTable,
    TypeInfo,
};
use std::{
    collections::HashMap,
    ffi::c_void,
    path::{Path, PathBuf},
    ptr::NonNull,
    sync::Arc,
};

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
    pub fn load(library_path: &Path, gc: Arc<GarbageCollector>) -> Result<Self, anyhow::Error> {
        let mut library = MunLibrary::new(library_path)?;

        let version = library.get_abi_version();
        if abi::ABI_VERSION != version {
            return Err(anyhow::anyhow!(
                "ABI version mismatch. munlib is `{}` but runtime is `{}`",
                version,
                abi::ABI_VERSION
            ));
        }

        let allocator_ptr = Arc::into_raw(gc.clone()) as *mut std::ffi::c_void;
        library.set_allocator_handle(allocator_ptr);

        let info = library.get_info();
        let assembly = Assembly {
            library_path: library_path.to_path_buf(),
            library: library.into_inner(),
            legacy_libs: Vec::new(),
            info,
            allocator: gc,
        };

        Ok(assembly)
    }

    /// Private implementation of runtime linking
    fn link_all_impl<'a>(
        dispatch_table: &mut DispatchTable,
        to_link: impl Iterator<Item = (&'a mut *const c_void, &'a FunctionPrototype)>,
    ) -> anyhow::Result<()> {
        let mut to_link: Vec<_> = to_link.collect();

        let mut retry = true;
        while retry {
            retry = false;
            let mut failed_to_link = Vec::new();

            // Try to link outstanding entries
            for (dispatch_ptr, fn_prototype) in to_link.into_iter() {
                // Ensure that the function is in the runtime dispatch table
                if let Some(fn_def) = dispatch_table.get_fn(fn_prototype.name()) {
                    // Ensure that the function's signature is the same.
                    if fn_prototype.signature != fn_def.prototype.signature {
                        return Err(anyhow!("Failed to link: function '{}' is missing. A function with the same name does exist, but the signatures do not match.", fn_prototype.name()));
                        // (expected: {}, found: {}).", fn_prototype.name(), fn_prototype, fn_def.prototype));
                    }

                    *dispatch_ptr = fn_def.fn_ptr;
                    retry = true;
                } else {
                    failed_to_link.push((dispatch_ptr, fn_prototype));
                }
            }

            // Move all failed entries, for (potentially) another try
            to_link = failed_to_link;
        }

        if !to_link.is_empty() {
            for (_, fn_prototype) in to_link {
                error!(
                    "Failed to link: function `{}` is missing.",
                    fn_prototype.name()
                );
            }

            return Err(anyhow!("Failed to link due to missing dependencies."));
        }

        Ok(())
    }

    /// Tries to link the `assemblies`, resulting in a new [`DispatchTable`] on success. This leaves
    /// the original `dispatch_table` intact, in case of linking errors.
    pub(super) fn link_all<'a>(
        assemblies: impl Iterator<Item = &'a mut Assembly>,
        dispatch_table: &DispatchTable,
    ) -> anyhow::Result<DispatchTable> {
        let assemblies: Vec<&'a mut _> = assemblies.collect();

        // Clone the dispatch table, such that we can roll back if linking fails
        let mut dispatch_table = dispatch_table.clone();

        // Insert all assemblies' functions into the dispatch table
        for assembly in assemblies.iter() {
            for function in assembly.info().symbols.functions() {
                dispatch_table.insert_fn(function.prototype.name(), function.clone());
            }
        }

        let to_link = assemblies
            .into_iter()
            .flat_map(|asm| asm.info.dispatch_table.iter_mut())
            // Only take signatures into account that do *not* yet have a function pointer assigned
            // by the compiler.
            .filter(|(ptr, _)| ptr.is_null());

        Assembly::link_all_impl(&mut dispatch_table, to_link)?;

        Ok(dispatch_table)
    }

    /// Tries to link the `assemblies`, resulting in a new [`DispatchTable`] on success. This leaves
    /// the original `dispatch_table` intact, in case of linking errors.
    pub(super) fn relink_all(
        unlinked_assemblies: &mut HashMap<PathBuf, Assembly>,
        linked_assemblies: &mut HashMap<PathBuf, Assembly>,
        dispatch_table: &DispatchTable,
        type_table: &TypeTable,
    ) -> anyhow::Result<(DispatchTable, TypeTable)> {
        let mut assemblies = unlinked_assemblies
            .iter_mut()
            .map(|(old_path, asm)| {
                let old_assembly = linked_assemblies.get(old_path);

                (asm, old_assembly)
            })
            .collect::<Vec<_>>();

        // Clone the type table, such that we can roll back if linking fails
        let mut type_table = type_table.clone();

        // TODO: Link types

        // Clone the dispatch table, such that we can roll back if linking fails
        let mut dispatch_table = dispatch_table.clone();

        // Remove the old assemblies' functions from the dispatch table
        for (_, old_assembly) in assemblies.iter() {
            if let Some(assembly) = old_assembly {
                for function in assembly.info.symbols.functions() {
                    dispatch_table.remove_fn(function.prototype.name());
                }
            }
        }

        // Insert all assemblies' functions into the dispatch table
        for (new_assembly, _) in assemblies.iter() {
            for function in new_assembly.info().symbols.functions() {
                dispatch_table.insert_fn(function.prototype.name(), function.clone());
            }
        }

        let to_link = assemblies
            .iter_mut()
            .flat_map(|(asm, _)| asm.info.dispatch_table.iter_mut())
            // Only take signatures into account that do *not* yet have a function pointer assigned
            // by the compiler.
            .filter(|(ptr, _)| ptr.is_null());

        Assembly::link_all_impl(&mut dispatch_table, to_link)?;

        let mut newly_linked = HashMap::new();
        std::mem::swap(unlinked_assemblies, &mut newly_linked);

        for (old_path, mut new_assembly) in newly_linked.into_iter() {
            let mut old_assembly = linked_assemblies
                .remove(&old_path)
                .expect("Assembly must exist.");

            let new_path = if let Some(new_path) = assemblies_to_keep.remove(&old_path) {
                // Retain all existing legacy libs
                new_assembly
                    .legacy_libs
                    .append(&mut old_assembly.legacy_libs);

                new_assembly.legacy_libs.push(old_assembly.into_library());

                new_path
            } else {
                new_assembly.library_path().to_path_buf()
            };

            linked_assemblies.insert(new_path, new_assembly);
        }

        Ok(dispatch_table)
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

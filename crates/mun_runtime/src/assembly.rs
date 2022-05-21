use crate::{garbage_collector::GarbageCollector, DispatchTable};
use anyhow::anyhow;
use libloader::{MunLibrary, TempLibrary};
use log::error;
use memory::{
    mapping::{Mapping, MemoryMapper},
    type_table::TypeTable,
    TryFromAbiError, TypeInfo,
};
use std::{
    collections::{HashMap, VecDeque},
    ffi::c_void,
    path::{Path, PathBuf},
    sync::Arc,
};

/// An assembly is a hot reloadable compilation unit, consisting of one or more Mun modules.
pub struct Assembly {
    library_path: PathBuf,
    library: TempLibrary,
    info: abi::AssemblyInfo,
    allocator: Arc<GarbageCollector>,
}

impl Assembly {
    /// Loads an assembly and its information for the shared library at `library_path`.
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
            info,
            allocator: gc,
        };

        Ok(assembly)
    }

    fn link_all_types<'a>(
        type_table: &mut TypeTable,
        to_link: impl Iterator<Item = (&'a abi::TypeId, &'a mut *const c_void, &'a str)>,
    ) -> anyhow::Result<()> {
        let mut to_link: Vec<_> = to_link.collect();

        // Try to link all LUT entries
        let mut failed_to_link = false;
        for (type_id, type_info_ptr, debug_name) in to_link.into_iter() {
            // Ensure that the function is in the runtime dispatch table
            if let Some(type_info) = type_table.find_type_info_by_id(type_id) {
                *type_info_ptr = Arc::into_raw(type_info) as *const c_void;
            } else {
                dbg!(debug_name);
                failed_to_link = true;
            }
        }

        if failed_to_link {
            return Err(anyhow!("Failed to link types due to missing type dependencies."));
        }

        Ok(())
    }

    /// Private implementation of runtime linking
    fn link_all_functions<'a>(
        dispatch_table: &mut DispatchTable,
        type_table: &TypeTable,
        to_link: impl Iterator<Item = (&'a mut *const c_void, &'a abi::FunctionPrototype)>,
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
                    let type_ids = fn_def
                        .prototype
                        .signature
                        .arg_types
                        .iter()
                        .map(|type_info| &type_info.id);

                    // Ensure that the function's signature is the same.
                    if !fn_prototype.signature.arg_types().iter().eq(type_ids) {
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
        type_table: &TypeTable,
    ) -> anyhow::Result<(DispatchTable, TypeTable)> {
        let mut assemblies: Vec<&'a mut _> = assemblies.collect();

        // Clone the type table, such that we can roll back if linking fails
        let mut type_table = type_table.clone();

        // Collect all types that need to be loaded
        let mut types_to_load: VecDeque<&abi::TypeInfo> = assemblies
            .iter()
            .flat_map(|asm| asm.info().symbols.types().iter())
            .collect();

        // Load all types
        while let Some(type_info) = types_to_load.pop_front() {
            match TypeInfo::try_from_abi(type_info, &type_table) {
                Ok(type_info) => {
                    assert!(type_table.insert_type(Arc::new(type_info)).is_none())
                },
                Err(TryFromAbiError::UnknownTypeId(type_id)) => types_to_load.push_back(type_info),
            }
        }

        let types_to_link = assemblies
            .iter_mut()
            .flat_map(|asm| asm.info.type_lut.iter_mut())
            // Only take signatures into account that do *not* yet have a type handle assigned
            // by the compiler.
            .filter(|(_, ptr, _)| ptr.is_null());

        Assembly::link_all_types(&mut type_table, types_to_link)?;

        // Clone the dispatch table, such that we can roll back if linking fails
        let mut dispatch_table = dispatch_table.clone();

        // Insert all assemblies' functions into the dispatch table
        for assembly in assemblies.iter() {
            dispatch_table.insert_module(&assembly.info().symbols, &type_table);
        }

        let functions_to_link = assemblies
            .into_iter()
            .flat_map(|asm| asm.info.dispatch_table.iter_mut())
            // Only take signatures into account that do *not* yet have a function pointer assigned
            // by the compiler.
            .filter(|(ptr, _)| ptr.is_null());

        Assembly::link_all_functions(&mut dispatch_table, &type_table, functions_to_link)?;

        Ok((dispatch_table, type_table))
    }

    /// Tries to link the `unlinked_assemblies`, resulting in a new [`DispatchTable`] on success.
    /// This leaves the original `dispatch_table` intact, in case of linking errors.
    pub(super) fn relink_all(
        unlinked_assemblies: &mut HashMap<PathBuf, Assembly>,
        linked_assemblies: &mut HashMap<PathBuf, Assembly>,
        dispatch_table: &DispatchTable,
        type_table: &TypeTable,
    ) -> anyhow::Result<(DispatchTable, TypeTable)> {
        let mut dependencies: HashMap<String, Vec<String>> = unlinked_assemblies
            .values()
            .map(|assembly| {
                let info = &assembly.info;
                let dependencies: Vec<String> = info.dependencies().map(From::from).collect();

                (info.symbols.path().to_owned(), dependencies)
            })
            .filter(|(_, dependencies)| !dependencies.is_empty())
            .collect();

        // Associate the new assemblies with the old assemblies
        let mut assemblies_to_link: VecDeque<_> = unlinked_assemblies
            .iter_mut()
            .map(|(old_path, asm)| (linked_assemblies.get(old_path), asm))
            .collect();

        // Clone the type table, such that we can roll back if linking fails
        let mut type_table = type_table.clone();

        // Clone the dispatch table, such that we can roll back if linking fails
        let mut dispatch_table = dispatch_table.clone();

        while let Some(mut entry) = assemblies_to_link.pop_front() {
            let (ref old_assembly, ref mut new_assembly) = entry;

            let new_path = new_assembly.info.symbols.path();

            // Are there any dependencies that still need to be loaded?
            if dependencies.contains_key(new_path) {
                assemblies_to_link.push_back(entry);

                continue;
            }

            let old_types: Option<(&Assembly, Vec<Arc<TypeInfo>>)> =
                old_assembly.map(|old_assembly| {
                    // Remove the old assemblies' types from the type table
                    let old_types = old_assembly
                        .info
                        .symbols
                        .types()
                        .iter()
                        .map(|type_info| {
                            type_table.remove_type_by_id(&type_info.id).expect(
                                "All types from a loaded assembly must exist in the type table.",
                            )
                        })
                        .collect();

                    (old_assembly, old_types)
                });

            // Collect all types that need to be loaded
            let mut types_to_load: VecDeque<&abi::TypeInfo> =
                new_assembly.info.symbols.types().iter().collect();

            let mut new_types = Vec::with_capacity(types_to_load.len());

            // Load all types, retrying types that depend on other unloaded types within the module
            while let Some(type_info) = types_to_load.pop_front() {
                match TypeInfo::try_from_abi(type_info, &type_table) {
                    Ok(type_info) => {
                        let type_info = Arc::new(type_info);
                        new_types.push(type_info.clone());
                        assert!(type_table.insert_type(type_info).is_none());
                    },
                    Err(TryFromAbiError::UnknownTypeId(_)) => {
                        types_to_load.push_back(type_info);
                    }
                }
            }

            let types_to_link = new_assembly
                .info
                .type_lut
                .iter_mut()
                // Only take signatures into account that do *not* yet have a type handle assigned
                // by the compiler.
                .filter(|(_, ptr, _)| ptr.is_null());

            Assembly::link_all_types(&mut type_table, types_to_link)?;

            // Memory map allocated object
            if let Some((old_assembly, old_types)) = old_types {
                let mapping = Mapping::new(&old_types, &new_types);
                let _deleted_objects = old_assembly.allocator.map_memory(mapping);
                // DISCUSSION: Do we need to maintain an assembly for the type LUT of allocated objects with deleted types?
            }

            // Remove the old assembly's functions from the dispatch table
            if let Some(old_assembly) = old_assembly {
                dispatch_table.remove_module(&old_assembly.info.symbols);
            }

            // Insert the new assembly's functions into the dispatch table
            dispatch_table.insert_module(&new_assembly.info.symbols, &type_table);

            let functions_to_link = new_assembly
                .info
                .dispatch_table
                .iter_mut()
                // Only take signatures into account that do *not* yet have a function pointer assigned
                // by the compiler. When an assembly is compiled it "pre-fills" its internal dispatch
                // table with pointers to self-referencing functions.
                .filter(|(ptr, _)| ptr.is_null());

            // Update the dispatch tables of the assemblies themselves based on our global dispatch
            // table. This will effectively link the function definitions of the assemblies together.
            // It also modifies the internal state of the assemblies.
            //
            // Note that linking may fail because for instance functions remaining unlinked (missing)
            // or the signature of a function doesnt match.
            Assembly::link_all_functions(&mut dispatch_table, &type_table, functions_to_link)?;

            // Remove this assembly from the dependencies
            dependencies
                .values_mut()
                .for_each(|dependencies| dependencies.retain(|path| path != new_path));

            // Remove assemblies that no longer have dependencies
            dependencies.retain(|_, dependencies| !dependencies.is_empty());
        }

        let mut newly_linked = HashMap::new();
        std::mem::swap(unlinked_assemblies, &mut newly_linked);

        for (old_path, new_assembly) in newly_linked.into_iter() {
            assert!(
                linked_assemblies.remove(&old_path).is_some(),
                "Assembly must exist."
            );

            let new_path = new_assembly.library_path.clone();
            linked_assemblies.insert(new_path, new_assembly);
        }

        Ok((dispatch_table, type_table))
    }

    /// Returns the assembly's information.
    pub fn info(&self) -> &abi::AssemblyInfo {
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

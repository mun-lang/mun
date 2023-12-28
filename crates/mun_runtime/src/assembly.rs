use std::{
    collections::{HashMap, VecDeque},
    ffi::c_void,
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

use itertools::Itertools;
use log::error;

use mun_abi as abi;
use mun_libloader::{MunLibrary, TempLibrary};
use mun_memory::{
    mapping::{Mapping, MemoryMapper},
    type_table::TypeTable,
    Type,
};

use crate::{garbage_collector::GarbageCollector, DispatchTable};

/// An error that occurs upon loading of a Mun library.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("An assembly with the same name already exists")]
    AlreadyExists,
    #[error(transparent)]
    FailedToLoadSharedLibrary(#[from] mun_libloader::InitError),
    #[error("ABI version mismatch. munlib is `{actual}` but runtime is `{expected}`")]
    MismatchedAbiVersions { expected: u32, actual: u32 },
    #[error(transparent)]
    Other(#[from] io::Error),
}

/// An error that occurs upon linking of a Mun assembly.
#[derive(Debug, thiserror::Error)]
pub enum LinkError {
    /// Failed to load assembly
    #[error(transparent)]
    LoadAssembly(#[from] LoadError),
    /// Failed to load type
    #[error("Failed to load type with id `{0}`")]
    LoadType(String),
    /// Failed to link function
    #[error(transparent)]
    Function(#[from] LinkFunctionsError),
    /// Failed to link assembly's types
    #[error("Failed to link types: {0:?}")]
    MissingTypes(Vec<String>),
}

/// An error that occurs upon linking of a Mun function prototype.
#[derive(Debug, thiserror::Error)]
pub enum LinkFunctionsError {
    /// Failed to resolve function argument
    #[error("Could not resolve function `{fn_name}`'s argument type #{idx}: {type_id}")]
    UnresolvedArgument {
        /// Function name
        fn_name: String,
        /// Argument index
        idx: usize,
        /// Argument type ID
        type_id: String,
    },
    /// Failed to resolve function return type
    #[error("Could not resolve function `{fn_name}`'s result type: {type_id}")]
    UnresolvedResult {
        /// Function name
        fn_name: String,
        /// Result type ID
        type_id: String,
    },
    /// Failed to retrieve function pointer due to mismatched function signature
    #[error("The function signature in the dispatch table does not match.\nExpected:\n\tfn {expected}\n\nFound:\n\tfn {found}")]
    MismatchedSignature {
        /// Expected function signature
        expected: String,
        /// Function signature found in dispatch table
        found: String,
    },
    /// Failed to load functions due to missing dependencies.
    #[error("Missing dependencies for functions: {functions:?}")]
    MissingDependencies {
        /// Function names for which dependencies were missing
        functions: Vec<String>,
    },
}

/// An assembly is a hot reloadable compilation unit, consisting of one or more Mun modules.
pub struct Assembly {
    library_path: PathBuf,
    library: TempLibrary,
    info: abi::AssemblyInfo<'static>,
    allocator: Arc<GarbageCollector>,
}

impl Assembly {
    /// Loads an assembly and its information for the shared library at `library_path`. The
    /// resulting `Assembly` is ensured to be linkable.
    ///
    /// # Safety
    ///
    /// A munlib is simply a shared object. When a library is loaded, initialisation routines
    /// contained within it are executed. For the purposes of safety, the execution of these
    /// routines is conceptually the same calling an unknown foreign function and may impose
    /// arbitrary requirements on the caller for the call to be sound.
    ///
    /// Additionally, the callers of this function must also ensure that execution of the
    /// termination routines contained within the library is safe as well. These routines may be
    /// executed when the library is unloaded.
    ///
    /// See [`libloading::Library::new`] for more information.
    pub unsafe fn load(library_path: &Path, gc: Arc<GarbageCollector>) -> Result<Self, LoadError> {
        let mut library = MunLibrary::new(library_path)?;

        let version = library.get_abi_version();
        if abi::ABI_VERSION != version {
            return Err(LoadError::MismatchedAbiVersions {
                expected: abi::ABI_VERSION,
                actual: version,
            });
        }

        let allocator_ptr = Arc::into_raw(gc.clone()) as *mut std::ffi::c_void;
        library.set_allocator_handle(allocator_ptr);

        let assembly = Assembly {
            info: library.get_info(),
            library_path: library_path.to_path_buf(),
            library: library.into_inner(),
            allocator: gc,
        };

        Ok(assembly)
    }

    /// On failure, returns debug names of all missing types.
    fn link_all_types<'abi>(
        type_table: &TypeTable,
        to_link: impl Iterator<Item = (&'abi abi::TypeId<'abi>, &'abi mut *const c_void, &'abi str)>,
    ) -> Result<(), Vec<String>> {
        // Try to link all LUT entries
        let failed_to_link = to_link
            .filter_map(|(type_id, type_info_ptr, debug_name)| {
                // Ensure that the function is in the runtime dispatch table
                if let Some(ty) = type_table.find_type_info_by_id(type_id) {
                    *type_info_ptr = Type::into_raw(ty);
                    None
                } else {
                    Some(debug_name)
                }
            })
            .map(ToString::to_string)
            .collect::<Vec<String>>();

        if failed_to_link.is_empty() {
            Ok(())
        } else {
            Err(failed_to_link)
        }
    }

    /// Private implementation of runtime linking
    fn link_all_functions<'abi>(
        dispatch_table: &DispatchTable,
        type_table: &TypeTable,
        to_link: impl Iterator<Item = (&'abi mut *const c_void, &'abi abi::FunctionPrototype<'abi>)>,
    ) -> Result<(), LinkFunctionsError> {
        let mut to_link: Vec<_> = to_link.collect();

        let mut retry = true;
        while retry {
            retry = false;
            let mut failed_to_link = Vec::new();

            // Try to link outstanding entries
            for (dispatch_ptr, fn_prototype) in to_link {
                // Get the types of the function arguments
                let fn_proto_arg_type_infos = fn_prototype
                    .signature
                    .arg_types()
                    .iter()
                    .enumerate()
                    .map(|(idx, fn_arg_type_id)| {
                        type_table
                            .find_type_info_by_id(fn_arg_type_id)
                            .ok_or_else(|| LinkFunctionsError::UnresolvedArgument {
                                fn_name: fn_prototype.name().to_string(),
                                idx: idx + 1,
                                type_id: fn_arg_type_id.to_string(),
                            })
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                // Get the return type info
                let fn_proto_ret_type_info = type_table
                    .find_type_info_by_id(&fn_prototype.signature.return_type)
                    .ok_or_else(|| LinkFunctionsError::UnresolvedResult {
                        fn_name: fn_prototype.name().to_string(),
                        type_id: fn_prototype.signature.return_type.to_string(),
                    })?;

                // Ensure that the function is in the runtime dispatch table
                if let Some(existing_fn_def) = dispatch_table.get_fn(fn_prototype.name()) {
                    if fn_proto_arg_type_infos != existing_fn_def.prototype.signature.arg_types
                        || fn_proto_ret_type_info != existing_fn_def.prototype.signature.return_type
                    {
                        let expected = fn_proto_arg_type_infos
                            .iter()
                            .map(|ty| ty.name().to_owned())
                            .join(", ");
                        let found = existing_fn_def
                            .prototype
                            .signature
                            .arg_types
                            .iter()
                            .map(|ty| ty.name().to_owned())
                            .join(", ");

                        let fn_name = fn_prototype.name();

                        return Err(LinkFunctionsError::MismatchedSignature {
                            expected: format!(
                                "{fn_name}({expected}) -> {}",
                                fn_proto_ret_type_info.name()
                            ),
                            found: format!(
                                "{fn_name}({found}) -> {}",
                                existing_fn_def.prototype.signature.return_type.name()
                            ),
                        });
                    }

                    *dispatch_ptr = existing_fn_def.fn_ptr;
                    retry = true;
                } else {
                    failed_to_link.push((dispatch_ptr, fn_prototype));
                }
            }

            // Move all failed entries, for (potentially) another try
            to_link = failed_to_link;
        }

        if to_link.is_empty() {
            Ok(())
        } else {
            Err(LinkFunctionsError::MissingDependencies {
                functions: to_link
                    .into_iter()
                    .map(|(_, fn_prototype)| fn_prototype.name().to_string())
                    .collect(),
            })
        }
    }

    /// Tries to link the `assemblies`, resulting in a new [`DispatchTable`] on success. This leaves
    /// the original `dispatch_table` intact, in case of linking errors.
    pub(super) fn link_all<'a>(
        assemblies: impl Iterator<Item = &'a mut Assembly>,
        dispatch_table: &DispatchTable,
        type_table: &TypeTable,
    ) -> Result<(DispatchTable, TypeTable), LinkError> {
        let mut assemblies: Vec<&'a mut _> = assemblies.collect();

        // Load all types, this creates a new type table that contains the types loaded
        let (type_table, _) = Type::try_from_abi(
            assemblies
                .iter()
                .flat_map(|asm| asm.info().symbols.types().iter()),
            type_table.clone(),
        )
        .map_err(|e| LinkError::LoadType(e.to_string()))?;

        let types_to_link = assemblies
            .iter_mut()
            .flat_map(|asm| asm.info.type_lut.iter_mut())
            // Only take signatures into account that do *not* yet have a type handle assigned
            // by the compiler.
            .filter(|(_, ptr, _)| ptr.is_null());

        Assembly::link_all_types(&type_table, types_to_link).map_err(LinkError::MissingTypes)?;

        // Clone the dispatch table, such that we can roll back if linking fails
        let mut dispatch_table = dispatch_table.clone();

        // Insert all assemblies' functions into the dispatch table
        for assembly in assemblies.iter() {
            dispatch_table.insert_module(&assembly.info().symbols, &type_table);
        }

        let functions_to_link = assemblies
            .into_iter()
            .flat_map(|asm| asm.info_mut().dispatch_table.iter_mut())
            // Only take signatures into account that do *not* yet have a function pointer assigned
            // by the compiler.
            .filter(|(ptr, _)| ptr.is_null());

        Assembly::link_all_functions(&dispatch_table, &type_table, functions_to_link)?;

        // Collect remaining types
        Type::collect_unreferenced_type_data();

        Ok((dispatch_table, type_table))
    }

    /// Tries to link the `unlinked_assemblies`, resulting in a new [`DispatchTable`] on success.
    /// This leaves the original `dispatch_table` intact, in case of linking errors.
    pub(super) fn relink_all(
        unlinked_assemblies: &mut HashMap<PathBuf, Assembly>,
        linked_assemblies: &mut HashMap<PathBuf, Assembly>,
        dispatch_table: &DispatchTable,
        type_table: &TypeTable,
    ) -> Result<(DispatchTable, TypeTable), LinkError> {
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

            let new_path = new_assembly.info().symbols.path().to_owned();

            // Are there any dependencies that still need to be loaded?
            if dependencies.contains_key(&new_path) {
                assemblies_to_link.push_back(entry);

                continue;
            }

            let old_types: Option<(&Assembly, Vec<Type>)> = old_assembly.map(|old_assembly| {
                // Remove the old assemblies' types from the type table
                let old_types = old_assembly
                    .info()
                    .symbols
                    .types()
                    .iter()
                    .map(|type_info| {
                        type_table.remove_type_by_type_info(type_info).expect(
                            "All types from a loaded assembly must exist in the type table.",
                        )
                    })
                    .collect();

                (old_assembly, old_types)
            });

            // Collect all types that need to be loaded
            let (updated_type_table, new_types) =
                Type::try_from_abi(new_assembly.info.symbols.types(), type_table)
                    .map_err(|e| LinkError::LoadType(e.to_string()))?;
            type_table = updated_type_table;

            // Load all types, retrying types that depend on other unloaded types within the module
            let types_to_link = new_assembly
                .info_mut()
                .type_lut
                .iter_mut()
                // Only take signatures into account that do *not* yet have a type handle assigned
                // by the compiler.
                .filter(|(_, ptr, _)| ptr.is_null());

            Assembly::link_all_types(&type_table, types_to_link)
                .map_err(LinkError::MissingTypes)?;

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
                .info_mut()
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
            Assembly::link_all_functions(&dispatch_table, &type_table, functions_to_link)?;

            // Remove this assembly from the dependencies
            dependencies
                .values_mut()
                .for_each(|dependencies| dependencies.retain(|path| path != &new_path));

            // Remove assemblies that no longer have dependencies
            dependencies.retain(|_, dependencies| !dependencies.is_empty());
        }

        let mut newly_linked = HashMap::new();
        std::mem::swap(unlinked_assemblies, &mut newly_linked);

        for (old_path, new_assembly) in newly_linked {
            assert!(
                linked_assemblies.remove(&old_path).is_some(),
                "Assembly must exist."
            );

            let new_path = new_assembly.library_path.clone();
            linked_assemblies.insert(new_path, new_assembly);
        }

        // Collect types
        Type::collect_unreferenced_type_data();

        Ok((dispatch_table, type_table))
    }

    /// Returns the assembly's information.
    pub fn info(&self) -> &abi::AssemblyInfo<'_> {
        &self.info
    }

    /// Returns the assembly's information.
    pub fn info_mut(&mut self) -> &mut abi::AssemblyInfo<'_> {
        // HACK: We want to make sure that the assembly info never outlives self.
        unsafe { std::mem::transmute(&mut self.info) }
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

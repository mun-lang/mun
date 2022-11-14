//! The Mun Runtime
//!
//! The Mun Runtime provides functionality for automatically hot reloading Mun C ABI
//! compliant shared libraries.
#![warn(missing_docs)]

mod assembly;
#[macro_use]
mod garbage_collector;
mod adt;
mod array;
mod dispatch_table;
mod function_info;
mod marshal;
mod reflection;

use anyhow::Result;
use dispatch_table::DispatchTable;
use garbage_collector::GarbageCollector;
use log::{debug, error, info};
use mun_abi as abi;
use mun_memory::{
    gc::{self, Array, GcRuntime},
    type_table::TypeTable,
};
use mun_project::LOCKFILE_NAME;
use notify::{event::ModifyKind, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::{HashMap, VecDeque},
    ffi,
    ffi::c_void,
    fmt::{Debug, Display, Formatter},
    io,
    mem::ManuallyDrop,
    path::{Path, PathBuf},
    ptr::NonNull,
    sync::{
        mpsc::{channel, Receiver},
        Arc,
    },
};

pub use crate::{
    adt::{RootedStruct, StructRef},
    array::{ArrayRef, RootedArray},
    assembly::Assembly,
    function_info::{
        FunctionDefinition, FunctionPrototype, FunctionSignature, IntoFunctionDefinition,
    },
    marshal::Marshal,
    reflection::{ArgumentReflection, ReturnTypeReflection},
};
// Re-export some useful types so crates dont have to depend on mun_memory as well.
use crate::array::RawArray;
pub use mun_memory::{Field, FieldData, HasStaticType, PointerType, StructType, Type};

/// Options for the construction of a [`Runtime`].
pub struct RuntimeOptions {
    /// Path to the entry point library
    pub library_path: PathBuf,
    /// Custom type table used for the runtime
    pub type_table: TypeTable,
    /// Custom user injected functions
    pub user_functions: Vec<FunctionDefinition>,
}

/// Retrieve the allocator using the provided handle.
///
/// # Safety
///
/// The allocator must have been set using the `set_allocator_handle` call - exposed by the Mun
/// library.
unsafe fn get_allocator(alloc_handle: *mut ffi::c_void) -> Arc<GarbageCollector> {
    Arc::from_raw(alloc_handle as *const GarbageCollector)
}

/// Retrieve the `TypeInfo` using the provided handle.
///
/// # Safety
///
/// The type handle must have been returned from a call to [`Arc<TypeInfo>::into_raw`][into_raw].
unsafe fn get_type_info(type_handle: *const ffi::c_void) -> Type {
    Type::from_raw(type_handle)
}

extern "C" fn new(
    type_handle: *const ffi::c_void,
    alloc_handle: *mut ffi::c_void,
) -> *const *mut ffi::c_void {
    // SAFETY: The runtime always constructs and uses `Arc<TypeInfo>::into_raw` to set the type
    // type handles in the type LUT.
    let type_info = ManuallyDrop::new(unsafe { get_type_info(type_handle) });

    // Safety: `new` is only called from within Mun assemblies' core logic, so we are guaranteed
    // that the `Runtime` and its `GarbageCollector` still exist if this function is called, and
    // will continue to do so for the duration of this function.
    let allocator = ManuallyDrop::new(unsafe { get_allocator(alloc_handle) });

    // Safety: the Mun Compiler guarantees that `new` is never called with `ptr::null()`.
    let handle = allocator.as_ref().alloc(&type_info);

    handle.into()
}

extern "C" fn new_array(
    type_handle: *const ffi::c_void,
    length: usize,
    alloc_handle: *mut ffi::c_void,
) -> *const *mut ffi::c_void {
    // SAFETY: The runtime always constructs and uses `Arc<TypeInfo>::into_raw` to set the type
    // type handles in the type LUT.
    let type_info = ManuallyDrop::new(unsafe { get_type_info(type_handle) });

    // Safety: `new` is only called from within Mun assemblies' core logic, so we are guaranteed
    // that the `Runtime` and its `GarbageCollector` still exist if this function is called, and
    // will continue to do so for the duration of this function.
    let allocator = ManuallyDrop::new(unsafe { get_allocator(alloc_handle) });

    let handle = allocator.as_ref().alloc_array(&type_info, length);

    handle.as_raw().into()
}

/// A builder for the [`Runtime`].
pub struct RuntimeBuilder {
    options: RuntimeOptions,
}

impl RuntimeBuilder {
    /// Constructs a new `RuntimeBuilder` for the shared library at `library_path`.
    fn new<P: Into<PathBuf>>(library_path: P) -> Self {
        Self {
            options: RuntimeOptions {
                library_path: library_path.into(),
                type_table: Default::default(),
                user_functions: Default::default(),
            },
        }
    }

    /// Adds a custom user function to the dispatch table.
    pub fn insert_fn<S: Into<String>, F: IntoFunctionDefinition>(
        mut self,
        name: S,
        func: F,
    ) -> Self {
        self.options.user_functions.push(func.into(name));
        self
    }

    /// Constructs a [`Runtime`] with the builder's options.
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
    /// See [`Assembly::load`] for more information.
    pub unsafe fn finish(self) -> anyhow::Result<Runtime> {
        Runtime::new(self.options)
    }
}

/// A runtime for the Mun language.
///
/// # Logging
///
/// The runtime uses [log] as a logging facade, but does not install a logger. To produce log
/// output, you have to use a [logger implementation][log-impl] compatible with the facade.
///
/// [log]: https://docs.rs/log
/// [log-impl]: https://docs.rs/log/0.4.13/log/#available-logging-implementations
pub struct Runtime {
    assemblies: HashMap<PathBuf, Assembly>,
    /// Assemblies that have changed and thus need to be relinked. Maps the old to the (potentially) new path.
    assemblies_to_relink: VecDeque<(PathBuf, PathBuf)>,
    dispatch_table: DispatchTable,
    type_table: TypeTable,
    watcher: RecommendedWatcher,
    watcher_rx: Receiver<notify::Result<Event>>,
    renamed_files: HashMap<usize, PathBuf>,
    gc: Arc<GarbageCollector>,
}

impl Runtime {
    /// Constructs a new [`RuntimeBuilder`] to construct a new [`Runtime`] instance.
    pub fn builder<P: Into<PathBuf>>(library_path: P) -> RuntimeBuilder {
        RuntimeBuilder::new(library_path)
    }

    /// Constructs a new `Runtime` that loads the library at `library_path` and its
    /// dependencies. The `Runtime` contains a file watcher that is triggered with an interval
    /// of `dur`.
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
    /// See [`Assembly::load`] for more information.
    pub unsafe fn new(mut options: RuntimeOptions) -> anyhow::Result<Runtime> {
        let (tx, rx) = channel();

        let mut dispatch_table = DispatchTable::default();
        let type_table = options.type_table;

        // Add internal functions
        options.user_functions.push(IntoFunctionDefinition::into(
            new as extern "C" fn(*const ffi::c_void, *mut ffi::c_void) -> *const *mut ffi::c_void,
            "new",
        ));

        options.user_functions.push(IntoFunctionDefinition::into(
            new_array
                as extern "C" fn(
                    *const ffi::c_void,
                    usize,
                    *mut ffi::c_void,
                ) -> *const *mut ffi::c_void,
            "new_array",
        ));

        options.user_functions.into_iter().for_each(|fn_def| {
            dispatch_table.insert_fn(fn_def.prototype.name.clone(), Arc::new(fn_def));
        });

        let watcher: RecommendedWatcher = notify::recommended_watcher(move |res| {
            tx.send(res).expect("Failed to send filesystem event.")
        })?;
        let mut runtime = Runtime {
            assemblies: HashMap::new(),
            assemblies_to_relink: VecDeque::new(),
            dispatch_table,
            type_table,
            watcher,
            watcher_rx: rx,
            renamed_files: HashMap::new(),
            gc: Arc::new(self::garbage_collector::GarbageCollector::default()),
        };

        runtime.add_assembly(&options.library_path)?;
        Ok(runtime)
    }

    /// Adds an assembly corresponding to the library at `library_path`.
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
    /// See [`Assembly::load`] for more information.
    unsafe fn add_assembly(&mut self, library_path: &Path) -> anyhow::Result<()> {
        let library_path = library_path.canonicalize()?;
        if self.assemblies.contains_key(&library_path) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "An assembly with the same name already exists.",
            )
            .into());
        }

        let mut loaded = HashMap::new();
        let mut to_load = VecDeque::new();
        to_load.push_back(library_path);

        // Load all assemblies and their dependencies
        while let Some(library_path) = to_load.pop_front() {
            // A dependency can be added by multiple dependants, so check that we didn't load it yet
            if loaded.contains_key(&library_path) {
                continue;
            }

            let assembly = Assembly::load(&library_path, self.gc.clone())?;

            let parent = library_path.parent().expect("Invalid library path");
            let extension = library_path.extension();

            let dependencies: Vec<String> =
                assembly.info().dependencies().map(From::from).collect();
            loaded.insert(library_path.clone(), assembly);

            for dependency in dependencies {
                let mut library_path = parent.join(dependency);
                if let Some(extension) = extension {
                    library_path = library_path.with_extension(extension);
                }

                if !loaded.contains_key(&library_path) {
                    to_load.push_back(library_path);
                }
            }
        }

        (self.dispatch_table, self.type_table) =
            Assembly::link_all(loaded.values_mut(), &self.dispatch_table, &self.type_table)?;

        for (library_path, assembly) in loaded.into_iter() {
            self.watcher
                .watch(library_path.parent().unwrap(), RecursiveMode::NonRecursive)?;

            self.assemblies.insert(library_path, assembly);
        }

        Ok(())
    }

    /// Retrieves the function definition corresponding to `function_name`, if available.
    pub fn get_function_definition(&self, function_name: &str) -> Option<Arc<FunctionDefinition>> {
        // TODO: Verify that when someone tries to invoke a non-public function, it should fail.
        self.dispatch_table.get_fn(function_name)
    }

    /// Retrieves the type definition corresponding to `type_name`, if available.
    pub fn get_type_info_by_name(&self, type_name: &str) -> Option<Type> {
        self.type_table.find_type_info_by_name(type_name)
    }

    /// Retrieve the type information corresponding to the `type_id`, if available.
    pub fn get_type_info_by_id(&self, type_id: &abi::TypeId) -> Option<Type> {
        self.type_table.find_type_info_by_id(type_id)
    }

    /// Updates the state of the runtime. This includes checking for file changes, and reloading
    /// compiled assemblies.
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
    /// See [`Assembly::load`] for more information.
    pub unsafe fn update(&mut self) -> bool {
        fn is_lockfile(path: &Path) -> bool {
            path.file_name().expect("Invalid file path.") == LOCKFILE_NAME
        }

        unsafe fn relink_assemblies(
            runtime: &mut Runtime,
        ) -> anyhow::Result<(DispatchTable, TypeTable)> {
            let mut loaded = HashMap::new();
            let to_load = &mut runtime.assemblies_to_relink;

            info!("Relinking assemblies:");
            for (old_path, new_path) in to_load.iter() {
                info!(
                    "{} -> {}",
                    old_path.to_string_lossy(),
                    new_path.to_string_lossy()
                );
            }

            // Load all assemblies and their dependencies
            while let Some((old_path, new_path)) = to_load.pop_front() {
                // A dependency can be added by multiple dependants, so check that we didn't load it yet
                if loaded.contains_key(&old_path) {
                    continue;
                }

                let assembly = Assembly::load(&new_path, runtime.gc.clone())?;

                let parent = new_path.parent().expect("Invalid library path");
                let extension = new_path.extension();

                let dependencies: Vec<String> =
                    assembly.info().dependencies().map(From::from).collect();
                loaded.insert(old_path.clone(), assembly);

                for dependency in dependencies {
                    let mut library_path = parent.join(dependency);
                    if let Some(extension) = extension {
                        library_path = library_path.with_extension(extension);
                    }

                    if !loaded.contains_key(&library_path)
                        && !runtime.assemblies.contains_key(&library_path)
                    {
                        to_load.push_back((old_path.clone(), library_path));
                    }
                }
            }

            Assembly::relink_all(
                &mut loaded,
                &mut runtime.assemblies,
                &runtime.dispatch_table,
                &runtime.type_table,
            )
        }

        let mut requires_relink = false;
        while let Ok(Ok(event)) = self.watcher_rx.try_recv() {
            for path in event.paths {
                if is_lockfile(&path) {
                    match event.kind {
                        EventKind::Create(_) => debug!("Lockfile created"),
                        EventKind::Remove(_) => {
                            debug!("Lockfile deleted");

                            requires_relink = true;
                        }
                        _ => (),
                    }
                } else {
                    let path = path.canonicalize().unwrap_or_else(|_| {
                        panic!("Failed to canonicalize path: {}.", path.to_string_lossy())
                    });

                    match event.kind {
                        EventKind::Modify(ModifyKind::Name(_)) => {
                            let tracker = event.attrs.tracker().expect("Invalid RENAME event.");
                            if let Some(old_path) = self.renamed_files.remove(&tracker) {
                                self.assemblies_to_relink.push_back((old_path, path));
                                // on_file_changed(self, &old_path, &path);
                            } else {
                                self.renamed_files.insert(tracker, path);
                            }
                        }
                        EventKind::Modify(_) => {
                            // TODO: don't overwrite existing
                            self.assemblies_to_relink.push_back((path.clone(), path));
                        }
                        _ => (),
                    }
                }
            }
        }

        if requires_relink {
            if self.assemblies_to_relink.is_empty() {
                debug!("The compiler didn't write a munlib.");
            } else {
                match relink_assemblies(self) {
                    Ok((dispatch_table, type_table)) => {
                        info!("Succesfully reloaded assemblies.");

                        self.dispatch_table = dispatch_table;
                        self.type_table = type_table;
                        self.assemblies_to_relink.clear();

                        return true;
                    }
                    Err(e) => error!("Failed to relink assemblies, due to {}.", e),
                }
            }
        }

        false
    }

    /// Returns a shared reference to the runtime's garbage collector.
    ///
    /// We cannot return an `Arc` here, because the lifetime of data contained in `GarbageCollector`
    /// is dependent on the `Runtime`.
    pub fn gc(&self) -> &GarbageCollector {
        self.gc.as_ref()
    }

    /// Collects all memory that is no longer referenced by rooted objects. Returns `true` if memory
    /// was reclaimed, `false` otherwise. This behavior will likely change in the future.
    pub fn gc_collect(&self) -> bool {
        self.gc.collect()
    }

    /// Returns statistics about the garbage collector.
    pub fn gc_stats(&self) -> gc::Stats {
        self.gc.stats()
    }

    /// Constructs an array from an iterator
    pub fn construct_array<'t, T: 't + Marshal<'t> + HasStaticType, I: IntoIterator<Item = T>>(
        &'t self,
        iter: I,
    ) -> ArrayRef<'t, T>
    where
        I::IntoIter: ExactSizeIterator,
    {
        let iter = iter.into_iter();
        let element_type = T::type_info();
        let array_type = element_type.array_type();
        let array_capacity = iter
            .size_hint()
            .1
            .expect("iterator doesn't return upper bound");
        let mut array_handle = self.gc.alloc_array(&array_type, array_capacity);

        let mut element_ptr = array_handle.data().as_ptr();
        let element_stride = array_handle.element_stride();
        let mut size = 0;
        for (idx, element) in iter.enumerate() {
            debug_assert!(
                idx < array_capacity,
                "looks like the size_hint was not properly implemented"
            );

            // Safety: the element_ptr came from NonNull and is only moved forward.
            T::marshal_to_ptr(
                element,
                unsafe { NonNull::new_unchecked(element_ptr).cast() },
                element_type,
            );

            // Safety: we trust the element stride
            element_ptr = unsafe { element_ptr.add(element_stride) };

            // Count the number of elements written
            size = idx + 1;
        }

        // Safety: we just initialized all the elements, so this operation is safe.
        unsafe {
            array_handle.set_length(size);
        }

        ArrayRef::new(RawArray(array_handle.as_raw()), self)
    }
}

/// An error that might occur when calling a mun function from Rust.
pub struct InvokeErr<'name, T> {
    msg: String,
    function_name: &'name str,
    arguments: T,
}

impl<'name, T> Debug for InvokeErr<'name, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.msg)
    }
}

impl<'name, T> Display for InvokeErr<'name, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.msg)
    }
}

impl<'name, T: InvokeArgs> InvokeErr<'name, T> {
    /// Retries a function invocation once, resulting in a potentially successful
    /// invocation.
    // FIXME: `unwrap_or_else` does not compile for `StructRef`, due to
    // https://doc.rust-lang.org/nomicon/lifetime-mismatch.html#improperly-reduced-borrows
    pub fn retry<'r, 'o, Output>(self, runtime: &'r mut Runtime) -> Result<Output, Self>
    where
        Output: 'o + ReturnTypeReflection + Marshal<'o>,
        'r: 'o,
    {
        // Safety: The output of `retry_impl` is guaranteed to only contain a shared
        // reference.
        unsafe { self.retry_impl(runtime) }
    }

    /// Retries the function invocation until it succeeds, resulting in an output.
    // FIXME: `unwrap_or_else` does not compile for `StructRef`, due to
    // https://doc.rust-lang.org/nomicon/lifetime-mismatch.html#improperly-reduced-borrows
    pub fn wait<'r, 'o, Output>(mut self, runtime: &'r mut Runtime) -> Output
    where
        Output: 'o + ReturnTypeReflection + Marshal<'o>,
        'r: 'o,
    {
        // Safety: The output of `retry_impl` is guaranteed to only contain a shared
        // reference.
        let runtime = &*runtime;

        loop {
            self = match unsafe { self.retry_impl(runtime) } {
                Ok(output) => return output,
                Err(e) => e,
            };
        }
    }

    /// Inner implementation that retries a function invocation once, resulting in a
    /// potentially successful invocation. This is a workaround for:
    /// https://doc.rust-lang.org/nomicon/lifetime-mismatch.html
    ///
    /// # Safety
    ///
    /// When calling this function, you have to guarantee that `runtime` is mutably
    /// borrowed. The `Output` value can only contain a shared borrow of `runtime`.
    unsafe fn retry_impl<'r, 'o, Output>(self, runtime: &'r Runtime) -> Result<Output, Self>
    where
        Output: 'o + ReturnTypeReflection + Marshal<'o>,
        'r: 'o,
    {
        #[allow(clippy::cast_ref_to_mut)]
        let runtime = &mut *(runtime as *const Runtime as *mut Runtime);

        eprintln!("{}", self.msg);
        while !runtime.update() {
            // Wait until there has been an update that might fix the error
        }

        runtime.invoke(self.function_name, self.arguments)
    }
}

/// A trait that handles calling a certain function with a set of arguments. This trait is
/// implemented for tuples up to and including 20 elements.
pub trait InvokeArgs {
    /// Determines whether the specified function can be called with these arguments
    fn can_invoke<'runtime>(
        &self,
        runtime: &'runtime Runtime,
        signature: &FunctionSignature,
    ) -> Result<(), String>;

    /// Calls the specified function with these function arguments
    ///
    /// # Safety
    ///
    /// The `fn_ptr` is cast and invoked which might result in undefined behavior.
    unsafe fn invoke<ReturnType>(self, fn_ptr: *const c_void) -> ReturnType;
}

// Implement `InvokeTraits` for tuples up to and including 20 elements
seq_macro::seq!(N in 0..=20 {#(
seq_macro::seq!(I in 0..N {
    #[allow(clippy::extra_unused_lifetimes)]
    impl<'arg, #(T~I: ArgumentReflection + Marshal<'arg>,)*> InvokeArgs for (#(T~I,)*) {
        #[allow(unused_variables)]
        fn can_invoke<'runtime>(&self, runtime: &'runtime Runtime, signature: &FunctionSignature) -> Result<(), String> {
            let arg_types = &signature.arg_types;

            // Ensure the number of arguments match
            #[allow(clippy::len_zero)]
            if N != arg_types.len() {
                return Err(format!("Invalid return type. Expected: {}. Found: {}", N, arg_types.len()))
            }

            #(
            if arg_types[I] != self.I.type_info(runtime) {
                return Err(format!(
                    "Invalid argument type at index {}. Expected: {}. Found: {}.",
                    I,
                    self.I.type_info(runtime).name(),
                    arg_types[I].name(),
                ));
            }
            )*

            Ok(())
        }

        unsafe fn invoke<ReturnType>(self, fn_ptr: *const c_void) -> ReturnType {
            #[allow(clippy::type_complexity)]
            let function: fn(#(T~I::MunType,)*) -> ReturnType = core::mem::transmute(fn_ptr);
            function(#(self.I.marshal_into(),)*)
        }
    }
});
)*});

impl Runtime {
    /// Invokes the Mun function called `function_name` with the specified `arguments`.
    pub fn invoke<
        'runtime,
        'ret,
        'name,
        ReturnType: ReturnTypeReflection + Marshal<'ret> + 'ret,
        ArgTypes: InvokeArgs,
    >(
        &'runtime self,
        function_name: &'name str,
        arguments: ArgTypes,
    ) -> Result<ReturnType, InvokeErr<'name, ArgTypes>>
    where
        'runtime: 'ret,
    {
        // Get the function information from the runtime
        let function_info = match self.get_function_definition(function_name).ok_or_else(|| {
            format!(
                "failed to obtain function '{}', no such function exists.",
                function_name
            )
        }) {
            Ok(function_info) => function_info,
            Err(msg) => {
                return Err(InvokeErr {
                    msg,
                    function_name,
                    arguments,
                })
            }
        };

        // Validate the arguments
        match arguments.can_invoke(self, &function_info.prototype.signature) {
            Ok(_) => {}
            Err(msg) => {
                return Err(InvokeErr {
                    msg,
                    function_name,
                    arguments,
                })
            }
        };

        // Validate the return type
        if !ReturnType::accepts_type(&function_info.prototype.signature.return_type) {
            return Err(InvokeErr {
                msg: format!(
                    "unexpected return type, got '{}', expected '{}",
                    &function_info.prototype.signature.return_type.name(),
                    ReturnType::type_hint()
                ),
                function_name,
                arguments,
            });
        }

        let result: ReturnType::MunType = unsafe { arguments.invoke(function_info.fn_ptr) };
        Ok(Marshal::marshal_from(result, self))
    }
}

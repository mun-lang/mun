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
mod marshal;
mod reflection;

use anyhow::Result;
use garbage_collector::GarbageCollector;
use log::{debug, error, info};
use memory::gc::{self, GcRuntime};
use mun_project::LOCKFILE_NAME;
use notify::{RawEvent, RecommendedWatcher, RecursiveMode, Watcher};
use rustc_hash::FxHashMap;
use std::{
    collections::{HashMap, VecDeque},
    ffi, io, mem,
    path::{Path, PathBuf},
    ptr::NonNull,
    string::ToString,
    sync::{
        mpsc::{channel, Receiver},
        Arc,
    },
};

pub use crate::{
    adt::{RootedStruct, StructRef},
    array::{ArrayRef, RawArray},
    assembly::Assembly,
    garbage_collector::UnsafeTypeInfo,
    marshal::Marshal,
    reflection::{ArgumentReflection, ReturnTypeReflection},
};
use abi::FunctionSignature;
pub use abi::IntoFunctionDefinition;
use std::ffi::c_void;
use std::fmt::{Debug, Display, Formatter};

/// Options for the construction of a [`Runtime`].
pub struct RuntimeOptions {
    /// Path to the entry point library
    pub library_path: PathBuf,
    /// Custom user injected functions
    pub user_functions: Vec<(abi::FunctionDefinition, abi::FunctionDefinitionStorage)>,
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

extern "C" fn new(
    type_info: *const abi::TypeInfo,
    alloc_handle: *mut ffi::c_void,
) -> *const *mut ffi::c_void {
    // Safety: `new` is only called from within Mun assemblies' core logic, so we are guaranteed
    // that the `Runtime` and its `GarbageCollector` still exist if this function is called, and
    // will continue to do so for the duration of this function.
    let allocator = unsafe { get_allocator(alloc_handle) };
    // Safety: the Mun Compiler guarantees that `new` is never called with `ptr::null()`.
    let type_info = UnsafeTypeInfo::new(unsafe { NonNull::new_unchecked(type_info as *mut _) });
    let handle = allocator.alloc(type_info);

    // Prevent destruction of the allocator
    mem::forget(allocator);

    handle.into()
}

extern "C" fn new_array(
    type_info: *const abi::TypeInfo,
    length: usize,
    alloc_handle: *mut ffi::c_void,
) -> *const *mut ffi::c_void {
    // Safety: `new` is only called from within Mun assemblies' core logic, so we are guaranteed
    // that the `Runtime` and its `GarbageCollector` still exist if this function is called, and
    // will continue to do so for the duration of this function.
    let allocator = unsafe { get_allocator(alloc_handle) };
    // Safety: the Mun Compiler guarantees that `new` is never called with `ptr::null()`.
    let type_info = UnsafeTypeInfo::new(unsafe { NonNull::new_unchecked(type_info as *mut _) });

    let handle = allocator.alloc_array(type_info, length);

    // Prevent destruction of the allocator
    mem::forget(allocator);

    handle.into()
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
                user_functions: Default::default(),
            },
        }
    }

    /// Adds a custom user function to the dispatch table.
    pub fn insert_fn<S: AsRef<str>, F: abi::IntoFunctionDefinition>(
        mut self,
        name: S,
        func: F,
    ) -> Self {
        self.options.user_functions.push(func.into(name));
        self
    }

    /// Constructs a [`Runtime`] with the builder's options.
    pub fn finish(self) -> anyhow::Result<Runtime> {
        Runtime::new(self.options)
    }
}

type DependencyCounter = usize;
type Dependency<T> = (T, DependencyCounter);
type DependencyMap<T> = FxHashMap<String, Dependency<T>>;

/// A runtime dispatch table that maps full paths to function and struct information.
#[derive(Clone, Default)]
pub struct DispatchTable {
    functions: FxHashMap<String, abi::FunctionDefinition>,
    fn_dependencies: FxHashMap<String, DependencyMap<abi::FunctionPrototype>>,
}

impl DispatchTable {
    /// Retrieves the [`abi::FunctionDefinition`] corresponding to `fn_path`, if it exists.
    pub fn get_fn(&self, fn_path: &str) -> Option<&abi::FunctionDefinition> {
        self.functions.get(fn_path)
    }

    /// Inserts the `fn_info` for `fn_path` into the dispatch table.
    ///
    /// If the dispatch table already contained this `fn_path`, the value is updated, and the old
    /// value is returned.
    pub fn insert_fn<S: ToString>(
        &mut self,
        fn_path: S,
        fn_info: abi::FunctionDefinition,
    ) -> Option<abi::FunctionDefinition> {
        self.functions.insert(fn_path.to_string(), fn_info)
    }

    /// Removes and returns the `fn_info` corresponding to `fn_path`, if it exists.
    pub fn remove_fn<S: AsRef<str>>(&mut self, fn_path: S) -> Option<abi::FunctionDefinition> {
        self.functions.remove(fn_path.as_ref())
    }

    /// Adds `fn_path` from `assembly_path` as a dependency; incrementing its usage counter.
    pub fn add_fn_dependency<S: ToString, T: ToString>(
        &mut self,
        assembly_path: S,
        fn_path: T,
        fn_prototype: abi::FunctionPrototype,
    ) {
        let dependencies = self
            .fn_dependencies
            .entry(assembly_path.to_string())
            .or_default();

        let (_, counter) = dependencies
            .entry(fn_path.to_string())
            .or_insert((fn_prototype, 0));

        *counter += 1;
    }

    /// Removes `fn_path` from `assembly_path` as a dependency; decrementing its usage counter.
    pub fn remove_fn_dependency<S: AsRef<str>, T: AsRef<str>>(
        &mut self,
        assembly_path: S,
        fn_path: T,
    ) {
        if let Some(dependencies) = self.fn_dependencies.get_mut(assembly_path.as_ref()) {
            if let Some((key, (fn_sig, counter))) = dependencies.remove_entry(fn_path.as_ref()) {
                if counter > 1 {
                    dependencies.insert(key, (fn_sig, counter - 1));
                }
            }
        }
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
    watcher: RecommendedWatcher,
    watcher_rx: Receiver<RawEvent>,
    renamed_files: HashMap<u32, PathBuf>,
    gc: Arc<GarbageCollector>,
    _user_functions: Vec<abi::FunctionDefinitionStorage>,
}

impl Runtime {
    /// Constructs a new [`RuntimeBuilder`] to construct a new [`Runtime`] instance.
    pub fn builder<P: Into<PathBuf>>(library_path: P) -> RuntimeBuilder {
        RuntimeBuilder::new(library_path)
    }

    /// Constructs a new `Runtime` that loads the library at `library_path` and its
    /// dependencies. The `Runtime` contains a file watcher that is triggered with an interval
    /// of `dur`.
    pub fn new(mut options: RuntimeOptions) -> anyhow::Result<Runtime> {
        let (tx, rx) = channel();

        let mut dispatch_table = DispatchTable::default();

        // Add internal functions
        options.user_functions.push(IntoFunctionDefinition::into(
            new as extern "C" fn(*const abi::TypeInfo, *mut ffi::c_void) -> *const *mut ffi::c_void,
            "new",
        ));

        options.user_functions.push(IntoFunctionDefinition::into(
            new_array
                as extern "C" fn(
                    *const abi::TypeInfo,
                    usize,
                    *mut ffi::c_void,
                ) -> *const *mut ffi::c_void,
            "new_array",
        ));

        let mut storages = Vec::with_capacity(options.user_functions.len());
        for (info, storage) in options.user_functions.into_iter() {
            dispatch_table.insert_fn(info.prototype.name().to_string(), info);
            storages.push(storage)
        }

        let watcher: RecommendedWatcher = Watcher::new_raw(tx)?;
        let mut runtime = Runtime {
            assemblies: HashMap::new(),
            assemblies_to_relink: VecDeque::new(),
            dispatch_table,
            watcher,
            watcher_rx: rx,
            renamed_files: HashMap::new(),
            gc: Arc::new(self::garbage_collector::GarbageCollector::default()),
            _user_functions: storages,
        };

        runtime.add_assembly(&options.library_path)?;
        Ok(runtime)
    }

    /// Adds an assembly corresponding to the library at `library_path`.
    fn add_assembly(&mut self, library_path: &Path) -> anyhow::Result<()> {
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

        self.dispatch_table = Assembly::link_all(loaded.values_mut(), &self.dispatch_table)?;

        for (library_path, assembly) in loaded.into_iter() {
            self.watcher
                .watch(library_path.parent().unwrap(), RecursiveMode::NonRecursive)?;

            self.assemblies.insert(library_path, assembly);
        }

        Ok(())
    }

    /// Retrieves the function definition corresponding to `function_name`, if available.
    pub fn get_function_definition(&self, function_name: &str) -> Option<&abi::FunctionDefinition> {
        // TODO: Verify that when someone tries to invoke a non-public function, it should fail.
        self.dispatch_table.get_fn(function_name)
    }

    /// Retrieves the type definition corresponding to `type_name`, if available.
    pub fn get_type_info(&self, type_name: &str) -> Option<&abi::TypeInfo> {
        for assembly in self.assemblies.values() {
            for type_info in assembly.info().symbols.types().iter() {
                if type_info.name() == type_name {
                    return Some(type_info);
                }
            }
        }

        None
    }

    /// Updates the state of the runtime. This includes checking for file changes, and reloading
    /// compiled assemblies.
    pub fn update(&mut self) -> bool {
        fn is_lockfile(path: &Path) -> bool {
            path.file_name().expect("Invalid file path.") == LOCKFILE_NAME
        }

        fn relink_assemblies(runtime: &mut Runtime) -> anyhow::Result<DispatchTable> {
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
            )
        }

        while let Ok(event) = self.watcher_rx.try_recv() {
            if let Some(path) = event.path {
                let op = event.op.expect("Invalid event.");

                if is_lockfile(&path) {
                    if op.contains(notify::op::CREATE) {
                        debug!("Lockfile created");
                    }
                    if op.contains(notify::op::REMOVE) {
                        debug!("Lockfile deleted");

                        match relink_assemblies(self) {
                            Ok(table) => {
                                info!("Succesfully reloaded assemblies.");

                                self.dispatch_table = table;
                                self.assemblies_to_relink.clear();

                                return true;
                            }
                            Err(e) => error!("Failed to relink assemblies, due to {}.", e),
                        }
                    }
                } else {
                    let path = path.canonicalize().unwrap_or_else(|_| {
                        panic!("Failed to canonicalize path: {}.", path.to_string_lossy())
                    });

                    if op.contains(notify::op::RENAME) {
                        let cookie = event.cookie.expect("Invalid RENAME event.");
                        if let Some(old_path) = self.renamed_files.remove(&cookie) {
                            self.assemblies_to_relink.push_back((old_path, path));
                        // on_file_changed(self, &old_path, &path);
                        } else {
                            self.renamed_files.insert(cookie, path);
                        }
                    } else if op.contains(notify::op::WRITE) {
                        // TODO: don't overwrite existing
                        self.assemblies_to_relink.push_back((path.clone(), path));
                    }
                }
            }
        }

        false
    }

    /// Returns a shared reference to the runtime's garbage collector.
    ///
    /// We cannot return an `Arc` here, because the lifetime of data contained in `GarbageCollector`
    /// is dependent on the `Runtime`.
    pub fn gc(&self) -> &dyn GcRuntime<UnsafeTypeInfo> {
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
    impl<'arg, #(T #I: ArgumentReflection + Marshal<'arg>,)*> InvokeArgs for (#(T #I,)*) {
        #[allow(unused_variables)]
        fn can_invoke<'runtime>(&self, runtime: &'runtime Runtime, signature: &FunctionSignature) -> Result<(), String> {
            let arg_types = signature.arg_types();

            // Ensure the number of arguments match
            #[allow(clippy::len_zero)]
            if N != arg_types.len() {
                return Err(format!("invalid return type. Expected: {}. Found: {}", N, arg_types.len()))
            }

            #(
            crate::reflection::equals_argument_type(runtime, arg_types[I], &self.I)
                .map_err(|(expected, found)| {
                    format!(
                        "invalid argument type at index {}. Expected: {}. Found: {}.",
                        I,
                        expected,
                        found,
                    )
                })?;
            )*

            Ok(())
        }

        unsafe fn invoke<ReturnType>(self, fn_ptr: *const c_void) -> ReturnType {
            #[allow(clippy::type_complexity)]
            let function: fn(#(T #I::MunType,)*) -> ReturnType = core::mem::transmute(fn_ptr);
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
        match if let Some(return_type) = function_info.prototype.signature.return_type() {
            if !ReturnType::equals_type(return_type) {
                Err((
                    return_type.name(),
                    ReturnType::type_name(),
                ))
            }
            else {
                Ok(())
            }
        } else if <() as ReturnTypeReflection>::type_guid() != ReturnType::type_guid() {
            Err((
                <() as ReturnTypeReflection>::type_name(),
                ReturnType::type_name(),
            ))
        } else {
            Ok(())
        } {
            Ok(_) => {}
            Err((expected, found)) => {
                return Err(InvokeErr {
                    msg: format!(
                        "invalid return type. Expected: {}. Found: {}",
                        expected, found,
                    ),
                    function_name,
                    arguments,
                })
            }
        }

        let result: ReturnType::MunType = unsafe { arguments.invoke(function_info.fn_ptr) };
        Ok(Marshal::marshal_from(result, self))
    }
}

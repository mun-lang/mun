//! The Mun Runtime
//!
//! The Mun Runtime provides functionality for automatically hot reloading Mun C ABI
//! compliant shared libraries.
#![warn(missing_docs)]

mod assembly;
#[macro_use]
mod macros;
#[macro_use]
mod garbage_collector;
mod adt;
mod marshal;
mod reflection;

use anyhow::Result;
use ffi::OsString;
use garbage_collector::GarbageCollector;
use log::{debug, error, info};
use memory::gc::{self, GcRuntime};
use mun_project::LOCKFILE_NAME;
use notify::{RawEvent, RecommendedWatcher, RecursiveMode, Watcher};
use rustc_hash::FxHashMap;
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    ffi, io, mem,
    path::{Path, PathBuf},
    ptr::NonNull,
    rc::Rc,
    string::ToString,
    sync::{
        mpsc::{channel, Receiver},
        Arc,
    },
};

pub use crate::{
    adt::{RootedStruct, StructRef},
    assembly::Assembly,
    garbage_collector::UnsafeTypeInfo,
    marshal::Marshal,
    reflection::{ArgumentReflection, ReturnTypeReflection},
};
pub use abi::IntoFunctionDefinition;

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

/// A builder for the [`Runtime`].
pub struct RuntimeBuilder {
    options: RuntimeOptions,
}

impl RuntimeBuilder {
    /// Constructs a new `RuntimeBuilder` for the shared library at `library_path`.
    pub fn new<P: Into<PathBuf>>(library_path: P) -> Self {
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

    /// Spawns a [`Runtime`] with the builder's options.
    pub fn spawn(self) -> anyhow::Result<Rc<RefCell<Runtime>>> {
        Runtime::new(self.options).map(|runtime| Rc::new(RefCell::new(runtime)))
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
                let mut library_path = PathBuf::from(parent.join(dependency));
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
            path.file_name().expect("Invalid file path.") == OsString::from(LOCKFILE_NAME)
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
                    let mut library_path = PathBuf::from(parent.join(dependency));
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
                    let path = path.canonicalize().expect(&format!(
                        "Failed to canonicalize path: {}.",
                        path.to_string_lossy()
                    ));

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

invoke_fn_impl! {
    fn invoke_fn0() -> InvokeErr0;
    fn invoke_fn1(arg1: A) -> InvokeErr1;
    fn invoke_fn2(arg1: A, arg2: B) -> InvokeErr2;
    fn invoke_fn3(arg1: A, arg2: B, arg3: C) -> InvokeErr3;
    fn invoke_fn4(arg1: A, arg2: B, arg3: C, arg4: D) -> InvokeErr4;
    fn invoke_fn5(arg1: A, arg2: B, arg3: C, arg4: D, arg5: E) -> InvokeErr5;
    fn invoke_fn6(arg1: A, arg2: B, arg3: C, arg4: D, arg5: E, arg6: F) -> InvokeErr6;
    fn invoke_fn7(arg1: A, arg2: B, arg3: C, arg4: D, arg5: E, arg6: F, arg7: G) -> InvokeErr7;
    fn invoke_fn8(arg1: A, arg2: B, arg3: C, arg4: D, arg5: E, arg6: F, arg7: G, arg8: H) -> InvokeErr8;
    fn invoke_fn9(arg1: A, arg2: B, arg3: C, arg4: D, arg5: E, arg6: F, arg7: G, arg8: H, arg9: I) -> InvokeErr9;
    fn invoke_fn10(arg1: A, arg2: B, arg3: C, arg4: D, arg5: E, arg6: F, arg7: G, arg8: H, arg9: I, arg10: J) -> InvokeErr10;
    fn invoke_fn11(arg1: A, arg2: B, arg3: C, arg4: D, arg5: E, arg6: F, arg7: G, arg8: H, arg9: I, arg10: J, arg11: K) -> InvokeErr11;
    fn invoke_fn12(arg1: A, arg2: B, arg3: C, arg4: D, arg5: E, arg6: F, arg7: G, arg8: H, arg9: I, arg10: J, arg11: K, arg12: L) -> InvokeErr12;
    fn invoke_fn13(arg1: A, arg2: B, arg3: C, arg4: D, arg5: E, arg6: F, arg7: G, arg8: H, arg9: I, arg10: J, arg11: K, arg12: L, arg13: M) -> InvokeErr13;
    fn invoke_fn14(arg1: A, arg2: B, arg3: C, arg4: D, arg5: E, arg6: F, arg7: G, arg8: H, arg9: I, arg10: J, arg11: K, arg12: L, arg13: M, arg14: N) -> InvokeErr14;
    fn invoke_fn15(arg1: A, arg2: B, arg3: C, arg4: D, arg5: E, arg6: F, arg7: G, arg8: H, arg9: I, arg10: J, arg11: K, arg12: L, arg13: M, arg14: N, arg15: O) -> InvokeErr15;
}

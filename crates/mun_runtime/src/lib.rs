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

use anyhow::Error;
use garbage_collector::GarbageCollector;
use memory::gc::{self, GcRuntime};
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use rustc_hash::FxHashMap;
use std::{
    cell::RefCell,
    collections::HashMap,
    ffi, io, mem,
    path::{Path, PathBuf},
    ptr::NonNull,
    rc::Rc,
    string::ToString,
    sync::{
        mpsc::{channel, Receiver},
        Arc,
    },
    time::Duration,
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
    /// Delay during which filesystem events are collected, deduplicated, and after which emitted.
    pub delay: Duration,
    /// Custom user injected functions
    pub user_functions: Vec<(
        abi::FunctionDefinition,
        abi::FunctionDefinitionStorage<'static>,
    )>,
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
                delay: Duration::from_millis(10),
                user_functions: Default::default(),
            },
        }
    }

    /// Sets the `delay`.
    pub fn set_delay(mut self, delay: Duration) -> Self {
        self.options.delay = delay;
        self
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
    pub fn spawn(self) -> Result<Rc<RefCell<Runtime>>, Error> {
        Runtime::new(self.options).map(|runtime| Rc::new(RefCell::new(runtime)))
    }
}

type DependencyCounter = usize;
type Dependency<T> = (T, DependencyCounter);
type DependencyMap<T> = FxHashMap<String, Dependency<T>>;

/// A runtime dispatch table that maps full paths to function and struct information.
#[derive(Default)]
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

/// A runtime type table that maps type `Guid`s to type information.
#[derive(Clone, Default)]
pub struct TypeTable {
    types: FxHashMap<abi::Guid, abi::TypeInfo>,
}

impl TypeTable {
    /// Retrieves the [`abi::TypeInfo`] corresponding to `type_guid`, if it exists.
    pub fn find_type_by_guid(&self, type_guid: &abi::Guid) -> Option<&abi::TypeInfo> {
        self.types.get(type_guid)
    }

    /// Retrieves the [`abi::TypeInfo`] corresponding to `type_name`, if it exists.
    ///
    /// This is a very slow operation, as we potentially need to iterate all types.
    pub fn find_type_by_name(&self, type_name: &str) -> Option<&abi::TypeInfo> {
        self.types.values().find(|ty| ty.name() == type_name)
    }

    /// Inserts the `type_info` for `type_guid` into the type table.
    ///
    /// If the type table already contained this `type_guid`, the value is updated, and the old
    /// value is returned.
    pub fn insert_type(
        &mut self,
        type_guid: abi::Guid,
        type_info: abi::TypeInfo,
    ) -> Option<abi::TypeInfo> {
        self.types.insert(type_guid, type_info)
    }

    /// Removes and returns the `type_info` corresponding to `type_guid`, if it exists.
    pub fn remove_type(&mut self, type_guid: &abi::Guid) -> Option<abi::TypeInfo> {
        self.types.remove(type_guid)
    }
}

/// A runtime for the Mun language.
pub struct Runtime {
    assemblies: HashMap<PathBuf, Assembly>,
    dispatch_table: DispatchTable,
    type_table: TypeTable,
    watcher: RecommendedWatcher,
    watcher_rx: Receiver<DebouncedEvent>,
    gc: Arc<GarbageCollector>,
    _user_functions: Vec<abi::FunctionDefinitionStorage<'static>>,
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

impl Runtime {
    /// Constructs a new `Runtime` that loads the library at `library_path` and its
    /// dependencies. The `Runtime` contains a file watcher that is triggered with an interval
    /// of `dur`.
    pub fn new(mut options: RuntimeOptions) -> Result<Runtime, Error> {
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

        let watcher: RecommendedWatcher = Watcher::new(tx, options.delay)?;
        let mut runtime = Runtime {
            assemblies: HashMap::new(),
            dispatch_table,
            type_table: TypeTable::default(),
            watcher,
            watcher_rx: rx,
            gc: Arc::new(self::garbage_collector::GarbageCollector::default()),
            _user_functions: storages,
        };

        runtime.add_assembly(&options.library_path)?;
        Ok(runtime)
    }

    /// Adds an assembly corresponding to the library at `library_path`.
    fn add_assembly(&mut self, library_path: &Path) -> Result<(), Error> {
        let library_path = library_path.canonicalize()?;
        if self.assemblies.contains_key(&library_path) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "An assembly with the same name already exists.",
            )
            .into());
        }

        let mut assembly = Assembly::load(&library_path, self.gc.clone(), &self.dispatch_table)?;
        for dependency in assembly.info().dependencies() {
            self.add_assembly(Path::new(dependency))?;
        }
        assembly.link(&mut self.dispatch_table);

        self.watcher
            .watch(library_path.parent().unwrap(), RecursiveMode::NonRecursive)?;

        self.assemblies.insert(library_path, assembly);
        Ok(())
    }

    /// Retrieves the function definition corresponding to `function_name`, if available.
    pub fn get_function_definition(&self, function_name: &str) -> Option<&abi::FunctionDefinition> {
        self.dispatch_table.get_fn(function_name)
    }

    /// Retrieves the type definition corresponding to `type_guid`, if available.
    pub fn find_type_info_by_guid(&self, type_guid: &abi::Guid) -> Option<&abi::TypeInfo> {
        self.type_table.find_type_by_guid(type_guid)
    }

    /// Retrieves the type definition corresponding to `type_name`, if available.
    ///
    /// This is a very slow operation, as we potentially need to iterate all types.
    pub fn find_type_info_by_name(&self, type_name: &str) -> Option<&abi::TypeInfo> {
        self.type_table.find_type_by_name(type_name)
    }

    /// Updates the state of the runtime. This includes checking for file changes, and reloading
    /// compiled assemblies.
    pub fn update(&mut self) -> bool {
        while let Ok(event) = self.watcher_rx.try_recv() {
            use notify::DebouncedEvent::*;
            match event {
                Write(ref path) | Rename(_, ref path) | Create(ref path) => {
                    if let Some(assembly) = self.assemblies.get_mut(path) {
                        if let Err(e) =
                            assembly.swap(path, &mut self.dispatch_table, &mut self.type_table)
                        {
                            println!(
                                "An error occured while reloading assembly '{}': {:?}",
                                path.to_string_lossy(),
                                e
                            );
                        } else {
                            println!(
                                "Succesfully reloaded assembly: '{}'",
                                path.to_string_lossy()
                            );
                            return true;
                        }
                    }
                }
                _ => {}
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

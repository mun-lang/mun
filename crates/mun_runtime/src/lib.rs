//! The Mun Runtime
//!
//! The Mun Runtime provides functionality for automatically hot reloading Mun C ABI
//! compliant shared libraries.
#![warn(missing_docs)]

mod assembly;
#[macro_use]
mod macros;

#[cfg(test)]
mod test;

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

use abi::{FunctionInfo, FunctionSignature, Guid, Privacy, Reflection, StructInfo, TypeInfo};
use failure::Error;
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};

pub use crate::assembly::Assembly;
use std::alloc::Layout;
use std::ffi::CString;

/// Options for the construction of a [`Runtime`].
#[derive(Clone, Debug)]
pub struct RuntimeOptions {
    /// Path to the entry point library
    pub library_path: PathBuf,
    /// Delay during which filesystem events are collected, deduplicated, and after which emitted.
    pub delay: Duration,
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
            },
        }
    }

    /// Sets the `delay`.
    pub fn set_delay(&mut self, delay: Duration) -> &mut Self {
        self.options.delay = delay;
        self
    }

    /// Spawns a [`Runtime`] with the builder's options.
    pub fn spawn(self) -> Result<Runtime, Error> {
        Runtime::new(self.options)
    }
}

/// A runtime dispatch table that maps full paths to function and struct information.
#[derive(Default)]
pub struct DispatchTable {
    functions: HashMap<String, FunctionInfo>,
    structs: HashMap<String, StructInfo>,
}

impl DispatchTable {
    /// Retrieves the [`abi::FunctionInfo`] corresponding to `fn_path`, if it exists.
    pub fn get_fn(&self, fn_path: &str) -> Option<&FunctionInfo> {
        self.functions.get(fn_path)
    }

    /// Inserts the `fn_info` for `fn_path` into the dispatch table.
    ///
    /// If the dispatch table already contained this `fn_path`, the value is updated, and the old
    /// value is returned.
    pub fn insert_fn<T: std::string::ToString>(
        &mut self,
        fn_path: T,
        fn_info: FunctionInfo,
    ) -> Option<FunctionInfo> {
        self.functions.insert(fn_path.to_string(), fn_info)
    }

    /// Removes and returns the `fn_info` corresponding to `fn_path`, if it exists.
    pub fn remove_fn<T: AsRef<str>>(&mut self, fn_path: T) -> Option<FunctionInfo> {
        self.functions.remove(fn_path.as_ref())
    }

    /// Retrieves the [`StructInfo`] corresponding to `struct_path`, if it exists.
    pub fn get_struct<T: AsRef<str>>(&self, struct_path: T) -> Option<&StructInfo> {
        self.structs.get(struct_path.as_ref())
    }

    /// Inserts the `struct_info` for `struct_path` into the dispatch table.
    ///
    /// If the dispatch table already contained this `struct_path`, the value is updated, and the
    /// old value is returned.
    pub fn insert_struct<T: std::string::ToString>(
        &mut self,
        struct_path: T,
        struct_info: StructInfo,
    ) -> Option<StructInfo> {
        self.structs.insert(struct_path.to_string(), struct_info)
    }

    /// Removes and returns the `struct_info` corresponding to `struct_path`, if it exists.
    pub fn remove_struct<T: AsRef<str>>(&mut self, struct_path: T) -> Option<StructInfo> {
        self.structs.remove(struct_path.as_ref())
    }
}

/// A runtime for the Mun language.
pub struct Runtime {
    assemblies: HashMap<PathBuf, Assembly>,
    dispatch_table: DispatchTable,
    watcher: RecommendedWatcher,
    watcher_rx: Receiver<DebouncedEvent>,

    _name: CString,
    _u64_type: CString,
    _ptr_mut_u8_type: CString,
    _arg_types: Vec<abi::TypeInfo>,
    _ret_type: Box<abi::TypeInfo>,
}

extern "C" fn malloc(size: u64, alignment: u64) -> *mut u8 {
    unsafe {
        std::alloc::alloc(Layout::from_size_align(size as usize, alignment as usize).unwrap())
    }
}

impl Runtime {
    /// Constructs a new `Runtime` that loads the library at `library_path` and its
    /// dependencies. The `Runtime` contains a file watcher that is triggered with an interval
    /// of `dur`.
    pub fn new(options: RuntimeOptions) -> Result<Runtime, Error> {
        let (tx, rx) = channel();

        let name = CString::new("malloc").unwrap();
        let u64_type = CString::new("core::u64").unwrap();
        let ptr_mut_u8_type = CString::new("core::u8*").unwrap();

        let arg_types = vec![
            TypeInfo {
                guid: Guid {
                    b: md5::compute("core::u64").0,
                },
                name: u64_type.as_ptr(),
            },
            TypeInfo {
                guid: Guid {
                    b: md5::compute("core::u64").0,
                },
                name: u64_type.as_ptr(),
            },
        ];

        let ret_type = Box::new(TypeInfo {
            guid: Guid {
                b: md5::compute("core::u8*").0,
            },
            name: ptr_mut_u8_type.as_ptr(),
        });

        let fn_info = FunctionInfo {
            signature: FunctionSignature {
                name: name.as_ptr(),
                arg_types: arg_types.as_ptr(),
                return_type: ret_type.as_ref(),
                num_arg_types: 2,
                privacy: Privacy::Public,
            },
            fn_ptr: malloc as *const std::ffi::c_void,
        };

        let mut dispatch_table = DispatchTable::default();
        dispatch_table.insert_fn("malloc", fn_info);

        let watcher: RecommendedWatcher = Watcher::new(tx, options.delay)?;
        let mut runtime = Runtime {
            assemblies: HashMap::new(),
            dispatch_table,
            watcher,
            watcher_rx: rx,

            _name: name,
            _u64_type: u64_type,
            _ptr_mut_u8_type: ptr_mut_u8_type,
            _arg_types: arg_types,
            _ret_type: ret_type,
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

        let mut assembly = Assembly::load(&library_path, &mut self.dispatch_table)?;
        for dependency in assembly.info().dependencies() {
            self.add_assembly(Path::new(dependency))?;
        }
        assembly.link(&self.dispatch_table)?;

        self.watcher
            .watch(library_path.parent().unwrap(), RecursiveMode::NonRecursive)?;

        self.assemblies.insert(library_path, assembly);
        Ok(())
    }

    /// Retrieves the function information corresponding to `function_name`, if available.
    pub fn get_function_info(&self, function_name: &str) -> Option<&FunctionInfo> {
        self.dispatch_table.get_fn(function_name)
    }

    /// Retrieves the struct information corresponding to `struct_name`, if available.
    pub fn get_struct_info(&self, struct_name: &str) -> Option<&StructInfo> {
        self.dispatch_table.get_struct(struct_name)
    }

    /// Updates the state of the runtime. This includes checking for file changes, and reloading
    /// compiled assemblies.
    pub fn update(&mut self) -> bool {
        while let Ok(event) = self.watcher_rx.try_recv() {
            use notify::DebouncedEvent::*;
            match event {
                Write(ref path) | Rename(_, ref path) | Create(ref path) => {
                    if let Some(assembly) = self.assemblies.get_mut(path) {
                        if let Err(e) = assembly.swap(path, &mut self.dispatch_table) {
                            println!(
                                "An error occured while reloading assembly '{}': {:?}",
                                path.to_string_lossy(),
                                e
                            );
                        } else {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }
}

/// Extends a result object with functions that allow retrying of an action.
pub trait RetryResultExt: Sized {
    /// Output type on success
    type Output;

    /// Retries an action, resulting in a potentially mutated version of itself.
    fn retry(self) -> Self;

    /// Keeps retrying the same action until it succeeds, resulting in an output.
    fn wait(self) -> Self::Output;
}

invoke_fn_impl! {
    fn invoke_fn0() -> InvokeErr0;
    fn invoke_fn1(a: A) -> InvokeErr1;
    fn invoke_fn2(a: A, b: B) -> InvokeErr2;
    fn invoke_fn3(a: A, b: B, c: C) -> InvokeErr3;
    fn invoke_fn4(a: A, b: B, c: C, d: D) -> InvokeErr4;
    fn invoke_fn5(a: A, b: B, c: C, d: D, e: E) -> InvokeErr5;
    fn invoke_fn6(a: A, b: B, c: C, d: D, e: E, f: F) -> InvokeErr6;
    fn invoke_fn7(a: A, b: B, c: C, d: D, e: E, f: F, g: G) -> InvokeErr7;
    fn invoke_fn8(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H) -> InvokeErr8;
    fn invoke_fn9(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I) -> InvokeErr9;
    fn invoke_fn10(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J) -> InvokeErr10;
    fn invoke_fn11(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J, k: K) -> InvokeErr11;
    fn invoke_fn12(a: A, b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J, k: K, l: L) -> InvokeErr12;
}

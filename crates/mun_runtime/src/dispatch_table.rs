use memory::type_table::TypeTable;
use rustc_hash::FxHashMap;

use crate::function_info::{FunctionDefinition, FunctionPrototype};

type DependencyCounter = usize;
type Dependency<T> = (T, DependencyCounter);
type DependencyMap<T> = FxHashMap<String, Dependency<T>>;

/// A runtime dispatch table that maps full paths to function and struct information.
#[derive(Clone, Default)]
pub struct DispatchTable {
    functions: FxHashMap<String, FunctionDefinition>,
    fn_dependencies: FxHashMap<String, DependencyMap<FunctionPrototype>>,
}

impl DispatchTable {
    /// Retrieves the [`FunctionDefinition`] corresponding to `fn_path`, if it exists.
    pub fn get_fn(&self, fn_path: &str) -> Option<&FunctionDefinition> {
        self.functions.get(fn_path)
    }

    /// Inserts the `fn_info` for `fn_path` into the dispatch table.
    ///
    /// If the dispatch table already contained this `fn_path`, the value is updated, and the old
    /// value is returned.
    pub fn insert_fn<S: ToString>(
        &mut self,
        fn_path: S,
        fn_info: FunctionDefinition,
    ) -> Option<FunctionDefinition> {
        self.functions.insert(fn_path.to_string(), fn_info)
    }

    /// Removes and returns the `fn_info` corresponding to `fn_path`, if it exists.
    pub fn remove_fn<S: AsRef<str>>(&mut self, fn_path: S) -> Option<FunctionDefinition> {
        self.functions.remove(fn_path.as_ref())
    }

    /// Removes the function definitions from the given assembly from this dispatch table.
    pub fn remove_module(&mut self, assembly: &abi::ModuleInfo) {
        for function in assembly.functions() {
            if let Some(value) = self.functions.get(function.prototype.name()) {
                if value.fn_ptr == function.fn_ptr {
                    self.functions.remove(function.prototype.name());
                }
            }
        }
    }

    /// Add the function definitions from the given assembly from this dispatch table.
    pub fn insert_module(&mut self, assembly: &abi::ModuleInfo, type_table: &TypeTable) {
        for fn_def in assembly.functions() {
            let fn_def = FunctionDefinition::try_from_abi(fn_def, type_table)
                .expect("All types from a loaded assembly must exist in the type table.");

            self.insert_fn(fn_def.prototype.name.clone(), fn_def);
        }
    }

    /// Adds `fn_path` from `assembly_path` as a dependency; incrementing its usage counter.
    pub fn add_fn_dependency<S: ToString, T: ToString>(
        &mut self,
        assembly_path: S,
        fn_path: T,
        fn_prototype: FunctionPrototype,
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

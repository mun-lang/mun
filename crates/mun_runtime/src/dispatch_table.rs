use crate::function_info::FunctionDefinition;
use mun_abi as abi;
use mun_memory::type_table::TypeTable;
use rustc_hash::FxHashMap;
use std::sync::Arc;

/// A runtime dispatch table that maps full paths to function and struct information.
#[derive(Clone, Default)]
pub struct DispatchTable {
    functions: FxHashMap<String, Arc<FunctionDefinition>>,
}

impl DispatchTable {
    /// Retrieves the [`FunctionDefinition`] corresponding to `fn_path`, if it exists.
    pub fn get_fn(&self, fn_path: &str) -> Option<Arc<FunctionDefinition>> {
        self.functions.get(fn_path).cloned()
    }

    /// Retrieves the name of all available functions.
    pub fn get_fn_names(&self) -> impl Iterator<Item = &str> {
        self.functions.keys().map(String::as_str)
    }

    /// Inserts the `fn_info` for `fn_path` into the dispatch table.
    ///
    /// If the dispatch table already contained this `fn_path`, the value is updated, and the old
    /// value is returned.
    pub fn insert_fn<S: ToString>(
        &mut self,
        fn_path: S,
        fn_info: Arc<FunctionDefinition>,
    ) -> Option<Arc<FunctionDefinition>> {
        self.functions.insert(fn_path.to_string(), fn_info)
    }

    // /// Removes and returns the `fn_info` corresponding to `fn_path`, if it exists.
    // pub fn remove_fn<S: AsRef<str>>(&mut self, fn_path: S) -> Option<Arc<FunctionDefinition>> {
    //     self.functions.remove(fn_path.as_ref())
    // }

    /// Removes the function definitions from the given assembly from this dispatch table.
    pub fn remove_module(&mut self, assembly: &abi::ModuleInfo<'_>) {
        for function in assembly.functions() {
            if let Some(value) = self.functions.get(function.prototype.name()) {
                if value.fn_ptr == function.fn_ptr {
                    self.functions.remove(function.prototype.name());
                }
            }
        }
    }

    /// Add the function definitions from the given assembly from this dispatch table.
    pub fn insert_module(&mut self, assembly: &abi::ModuleInfo<'_>, type_table: &TypeTable) {
        for fn_def in assembly.functions() {
            let fn_def = FunctionDefinition::try_from_abi(fn_def, type_table)
                .expect("All types from a loaded assembly must exist in the type table.");

            self.insert_fn(fn_def.prototype.name.clone(), Arc::new(fn_def));
        }
    }
}

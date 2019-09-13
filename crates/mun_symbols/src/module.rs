use std::collections::HashMap;

use crate::prelude::*;

/// Reflection information about a module.
#[derive(Debug)]
pub struct ModuleInfo {
    path: String,
    fields: HashMap<&'static str, &'static FieldInfo>,
    methods: HashMap<&'static str, &'static MethodInfo>,
    modules: HashMap<&'static str, &'static ModuleInfo>,
}

// TODO: How to resolve generic fields and methods?
impl ModuleInfo {
    /// Constructs a new `ModuleInfo`.
    pub fn new(
        path: &str,
        fields: &[&'static FieldInfo],
        methods: &[&'static MethodInfo],
        modules: &[&'static ModuleInfo],
    ) -> Self {
        let fields = {
            let mut map = HashMap::new();
            for field in fields.iter() {
                map.insert(field.name.as_ref(), *field);
            }
            map
        };

        let methods = {
            let mut map = HashMap::new();
            for method in methods.iter() {
                map.insert(method.name.as_ref(), *method);
            }
            map
        };

        let modules = {
            let mut map = HashMap::new();
            for module in modules.iter() {
                map.insert(module.path(), *module);
            }
            map
        };

        ModuleInfo {
            path: path.to_string(),
            fields,
            methods,
            modules,
        }
    }

    /// Retrieves the module's path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Finds the module's fields that match `filter`.
    pub fn find_fields<F>(&self, filter: F) -> impl Iterator<Item = &FieldInfo>
    where
        F: FnMut(&&FieldInfo) -> bool,
    {
        self.fields.values().map(|f| *f).filter(filter)
    }

    /// Retrieves the module's field with the specified `name`, if it exists.
    pub fn get_field(&self, name: &str) -> Option<&FieldInfo> {
        self.fields.get(name).map(|f| *f)
    }

    /// Retrieves the module's fields.
    pub fn get_fields(&self) -> impl Iterator<Item = &FieldInfo> {
        self.fields.values().map(|f| *f)
    }

    /// Finds the module's methods that match `filter`.
    pub fn find_methods<F>(&self, filter: F) -> impl Iterator<Item = &MethodInfo>
    where
        F: FnMut(&&MethodInfo) -> bool,
    {
        self.methods.values().map(|m| *m).filter(filter)
    }

    /// Retrieves the module's method with the specified `name`, if it exists.
    pub fn get_method(&self, name: &str) -> Option<&MethodInfo> {
        self.methods.get(name).map(|m| *m)
    }

    /// Retrieves the module's methods.
    pub fn get_methods(&self) -> impl Iterator<Item = &MethodInfo> {
        self.methods.values().map(|m| *m)
    }

    /// Finds the module's sub-modules that match `filter`.
    pub fn find_modules<F>(&self, filter: F) -> impl Iterator<Item = &ModuleInfo>
    where
        F: FnMut(&&ModuleInfo) -> bool,
    {
        self.modules.values().map(|m| *m).filter(filter)
    }

    /// Retrieves the module's sub-module with the specfied `name`.
    pub fn get_module(&self, name: &str) -> Option<&ModuleInfo> {
        self.modules.get(name).map(|m| *m)
    }

    /// Retrieves the module's sub-modules.
    pub fn get_modules(&self) -> impl Iterator<Item = &ModuleInfo> {
        self.modules.values().map(|m| *m)
    }
}

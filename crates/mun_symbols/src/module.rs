use std::collections::HashMap;

use crate::prelude::*;

#[derive(Debug)]
pub struct ModuleInfo {
    path: String,
    fields: HashMap<String, FieldInfo>,
    methods: HashMap<String, MethodInfo>,
    modules: HashMap<String, ModuleInfo>,
    structures: HashMap<String, StructureInfo>,
}

// TODO: How to resolve generic fields and methods?
impl ModuleInfo {
    pub fn new(
        path: &str,
        fields: HashMap<String, FieldInfo>,
        methods: HashMap<String, MethodInfo>,
        modules: HashMap<String, ModuleInfo>,
        structures: HashMap<String, StructureInfo>,
    ) -> Self {
        ModuleInfo {
            path: path.to_string(),
            fields,
            methods,
            modules,
            structures,
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn find_fields<F>(&self, filter: F) -> impl Iterator<Item = &FieldInfo>
    where
        F: FnMut(&&FieldInfo) -> bool,
    {
        self.fields.values().filter(filter)
    }

    pub fn get_field(&self, name: &str) -> Option<&FieldInfo> {
        self.fields.get(name)
    }

    pub fn get_fields(&self) -> impl Iterator<Item = &FieldInfo> {
        self.fields.values()
    }

    pub fn find_methods<F>(&self, filter: F) -> impl Iterator<Item = &MethodInfo>
    where
        F: FnMut(&&MethodInfo) -> bool,
    {
        self.methods.values().filter(filter)
    }

    pub fn get_method(&self, name: &str) -> Option<&MethodInfo> {
        self.methods.get(name)
    }

    pub fn get_methods(&self) -> impl Iterator<Item = &MethodInfo> {
        self.methods.values()
    }

    pub fn find_modules<F>(&self, filter: F) -> impl Iterator<Item = &ModuleInfo>
    where
        F: FnMut(&&ModuleInfo) -> bool,
    {
        self.modules.values().filter(filter)
    }

    pub fn get_module(&self, name: &str) -> Option<&ModuleInfo> {
        self.modules.get(name)
    }

    pub fn get_modules(&self) -> impl Iterator<Item = &ModuleInfo> {
        self.modules.values()
    }

    pub fn find_structures<F>(&self, filter: F) -> impl Iterator<Item = &StructureInfo>
    where
        F: FnMut(&&StructureInfo) -> bool,
    {
        self.structures.values().filter(filter)
    }

    pub fn get_structure(&self, name: &str) -> Option<&StructureInfo> {
        self.structures.get(name)
    }

    pub fn get_structures(&self) -> impl Iterator<Item = &StructureInfo> {
        self.structures.values()
    }
}

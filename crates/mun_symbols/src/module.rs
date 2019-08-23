use std::collections::HashMap;

use crate::prelude::*;

#[derive(Debug)]
pub struct ModuleInfo {
    path: String,
    fields: HashMap<&'static str, &'static FieldInfo>,
    methods: HashMap<&'static str, &'static MethodInfo>,
    modules: HashMap<&'static str, &'static ModuleInfo>,
}

// TODO: How to resolve generic fields and methods?
impl ModuleInfo {
    pub fn new(
        path: &str,
        fields: &[&'static FieldInfo],
        methods: &[&'static MethodInfo],
        modules: &[&'static ModuleInfo],
    ) -> Self {
        let fields = {
            let mut map = HashMap::new();
            for field in fields.iter() {
                map.insert(field.name(), *field);
            }
            map
        };
        
        let methods = {
            let mut map = HashMap::new();
            for method in methods.iter() {
                map.insert(method.name(), *method);
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

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn find_fields<F>(&self, filter: F) -> impl Iterator<Item = &FieldInfo>
    where
        F: FnMut(&&FieldInfo) -> bool,
    {
        self.fields.values().map(|f| *f).filter(filter)
    }

    pub fn get_field(&self, name: &str) -> Option<&FieldInfo> {
        self.fields.get(name).map(|f| *f)
    }

    pub fn get_fields(&self) -> impl Iterator<Item = &FieldInfo> {
        self.fields.values().map(|f| *f)
    }

    pub fn find_methods<F>(&self, filter: F) -> impl Iterator<Item = &MethodInfo>
    where
        F: FnMut(&&MethodInfo) -> bool,
    {
        self.methods.values().map(|m| *m).filter(filter)
    }

    pub fn get_method(&self, name: &str) -> Option<&MethodInfo> {
        self.methods.get(name).map(|m| *m)
    }

    pub fn get_methods(&self) -> impl Iterator<Item = &MethodInfo> {
        self.methods.values().map(|m| *m)
    }

    pub fn find_modules<F>(&self, filter: F) -> impl Iterator<Item = &ModuleInfo>
    where
        F: FnMut(&&ModuleInfo) -> bool,
    {
        self.modules.values().map(|m| *m).filter(filter)
    }

    pub fn get_module(&self, name: &str) -> Option<&ModuleInfo> {
        self.modules.get(name).map(|m| *m)
    }

    pub fn get_modules(&self) -> impl Iterator<Item = &ModuleInfo> {
        self.modules.values().map(|m| *m)
    }
}

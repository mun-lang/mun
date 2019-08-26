use crate::prelude::*;
use uuid::Uuid;

use std::any::{Any, TypeId};

// TODO: How to resolve generic fields and methods?
#[derive(Debug)]
pub struct TypeInfo {
    pub type_id: TypeId,
    pub uuid: Uuid,
    pub name: &'static str,
    pub fields: &'static [&'static FieldInfo],
    pub methods: &'static [&'static MethodInfo],
}

impl TypeInfo {
    pub fn find_fields(&self, filter: fn(&&FieldInfo) -> bool) -> impl Iterator<Item = &FieldInfo> {
        self.fields.iter().map(|f| *f).filter(filter)
    }

    pub fn get_field(&self, name: &str) -> Option<&FieldInfo> {
        self.fields.iter().find(|f| f.name() == name).map(|f| *f)
    }

    pub fn get_fields(&self) -> impl Iterator<Item = &FieldInfo> {
        self.fields.iter().map(|f| *f)
    }

    pub fn find_methods(
        &self,
        filter: fn(&&MethodInfo) -> bool,
    ) -> impl Iterator<Item = &MethodInfo> {
        self.methods.iter().map(|m| *m).filter(filter)
    }

    pub fn get_method(&self, name: &str) -> Option<&MethodInfo> {
        self.methods.iter().find(|f| f.name() == name).map(|f| *f)
    }

    pub fn get_methods(&self) -> impl Iterator<Item = &MethodInfo> {
        self.methods.iter().map(|m| *m)
    }
}

pub trait Reflectable: Any {
    fn reflect(&self) -> &'static TypeInfo {
        Self::type_info()
    }

    fn type_info() -> &'static TypeInfo;

    fn module_info() -> &'static ModuleInfo;
}

lazy_static! {
    static ref F32_TYPE_INFO: TypeInfo = TypeInfo {
        type_id: TypeId::of::<f32>(),
        uuid: Uuid::parse_str("fc4bacef-cd0e-4d58-8d4d-19504d58d87f").unwrap(),
        name: "f32",
        fields: &[],
        methods: &[],
    };
    static ref F64_TYPE_INFO: TypeInfo = TypeInfo {
        type_id: TypeId::of::<f64>(),
        uuid: Uuid::parse_str("fe58c2ab-f8db-4dab-80b1-578d871bc769").unwrap(),
        name: "f64",
        fields: &[],
        methods: &[],
    };
}

lazy_static! {
    static ref CORE_MODULE: ModuleInfo = { ModuleInfo::new("core", &[], &[], &[]) };
}

impl Reflectable for f32 {
    fn type_info() -> &'static TypeInfo {
        &F32_TYPE_INFO
    }

    fn module_info() -> &'static ModuleInfo {
        &CORE_MODULE
    }
}

impl Reflectable for f64 {
    fn type_info() -> &'static TypeInfo {
        &F64_TYPE_INFO
    }

    fn module_info() -> &'static ModuleInfo {
        &CORE_MODULE
    }
}

use crate::prelude::*;

use std::any::TypeId;

// TODO: How to resolve generic fields and methods?
#[derive(Debug)]
pub struct TypeInfo {
    pub type_id: TypeId,
    pub name: &'static str,
    pub fields: &'static [&'static FieldInfo],
    pub methods: &'static [&'static MethodInfo],
}

impl TypeInfo {
    pub fn find_fields(&self, filter: fn(&&FieldInfo)-> bool) -> impl Iterator<Item = &FieldInfo> {
        self.fields.iter().map(|f| *f).filter(filter)
    }

    pub fn get_field(&self, name: &str) -> Option<&FieldInfo> {
        self.fields.iter().find(|f| f.name() == name).map(|f| *f)
    }

    pub fn get_fields(&self) -> impl Iterator<Item = &FieldInfo> {
        self.fields.iter().map(|f| *f)
    }

    pub fn find_methods(&self, filter: fn(&&MethodInfo) -> bool) -> impl Iterator<Item = &MethodInfo> {
        self.methods.iter().map(|m| *m).filter(filter)
    }

    pub fn get_method(&self, name: &str) -> Option<&MethodInfo> {
        self.methods.iter().find(|f| f.name() == name).map(|f| *f)
    }

    pub fn get_methods(&self) -> impl Iterator<Item = &MethodInfo> {
        self.methods.iter().map(|m| *m)
    }
}

pub trait Reflectable {
    fn type_id() -> TypeId;
    fn type_info() -> &'static TypeInfo;

    fn module_info() -> &'static ModuleInfo;
}

const F32_TYPE_INFO: TypeInfo = TypeInfo {
    type_id: TypeId::of::<f32>(),
    name: "f32",
    fields: &[],
    methods: &[],
};

lazy_static! {
    static ref CORE_MODULE: ModuleInfo = {
        ModuleInfo::new("core", &[], &[], &[])
    };
}

impl Reflectable for f32 {
    fn type_id() -> TypeId {
        TypeId::of::<Self>()
    }

    fn type_info() -> &'static TypeInfo {
        &F32_TYPE_INFO
    }

    fn module_info() -> &'static ModuleInfo {
        &CORE_MODULE
    }
}

use crate::type_info::{HasStaticTypeInfo, TypeInfo};
use rustc_hash::FxHashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct TypeTable {
    type_id_to_type_info: FxHashMap<abi::TypeId, Arc<TypeInfo>>,
    type_name_to_type_info: FxHashMap<String, Arc<TypeInfo>>,
}

impl TypeTable {
    pub fn find_type_info_by_name<S: AsRef<str>>(&self, name: S) -> Option<Arc<TypeInfo>> {
        self.type_name_to_type_info
            .get(name.as_ref())
            .map(Clone::clone)
    }

    pub fn find_type_info_by_id(&self, type_id: &abi::TypeId) -> Option<Arc<TypeInfo>> {
        self.type_id_to_type_info.get(type_id).map(Clone::clone)
    }

    /// Inserts `type_info` into the type table.
    ///
    /// If the type table already contained this `type_info`, the value is updated, and the old
    /// value is returned.
    pub fn insert_type(&mut self, type_info: Arc<TypeInfo>) -> Option<Arc<TypeInfo>> {
        self.type_id_to_type_info
            .insert(type_info.id.clone(), type_info.clone());
        self.type_name_to_type_info
            .insert(type_info.name.clone(), type_info)
    }

    /// Removes and returns the `TypeInfo` corresponding to `type_info`, if it exists.
    pub fn remove_type(&mut self, type_info: &TypeInfo) -> Option<Arc<TypeInfo>> {
        self.type_id_to_type_info.remove(&type_info.id);
        self.type_name_to_type_info.remove(&type_info.name)
    }

    /// Removes and returns the `TypeInfo` corresponding to `type_id`, if it exists.
    pub fn remove_type_by_id(&mut self, type_id: &abi::TypeId) -> Option<Arc<TypeInfo>> {
        let type_info = self.type_id_to_type_info.remove(type_id);
        if let Some(type_info) = &type_info {
            self.type_name_to_type_info.remove(&type_info.name);
        }
        type_info
    }

    /// Removes and returns the `TypeInfo` corresponding to `name`, if it exists.
    pub fn remove_type_by_name<S: AsRef<str>>(&mut self, name: S) -> Option<Arc<TypeInfo>> {
        let type_info = self.type_name_to_type_info.remove(name.as_ref());
        if let Some(type_info) = &type_info {
            self.type_id_to_type_info.remove(&type_info.id);
        }
        type_info
    }
}

impl Default for TypeTable {
    fn default() -> Self {
        let mut type_table = Self {
            type_id_to_type_info: Default::default(),
            type_name_to_type_info: Default::default(),
        };

        type_table.insert_type(i8::type_info().clone());
        type_table.insert_type(i16::type_info().clone());
        type_table.insert_type(i32::type_info().clone());
        type_table.insert_type(i64::type_info().clone());
        type_table.insert_type(i128::type_info().clone());
        type_table.insert_type(u8::type_info().clone());
        type_table.insert_type(u16::type_info().clone());
        type_table.insert_type(u32::type_info().clone());
        type_table.insert_type(u64::type_info().clone());
        type_table.insert_type(u128::type_info().clone());
        type_table.insert_type(f32::type_info().clone());
        type_table.insert_type(f64::type_info().clone());
        type_table.insert_type(bool::type_info().clone());
        type_table.insert_type(<()>::type_info().clone());

        type_table
    }
}

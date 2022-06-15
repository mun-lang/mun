use std::sync::Arc;

use rustc_hash::FxHashMap;

use crate::type_info::{HasStaticTypeInfo, TypeInfo};

#[derive(Clone)]
pub struct TypeTable {
    type_id_to_type_info: FxHashMap<abi::TypeId, Arc<TypeInfo>>,
    type_name_to_type_info: FxHashMap<String, (abi::TypeId, Arc<TypeInfo>)>,
}

impl TypeTable {
    /// Returns the TypeInfo for the type with the given name.
    pub fn find_type_info_by_name<S: AsRef<str>>(&self, name: S) -> Option<Arc<TypeInfo>> {
        self.type_name_to_type_info
            .get(name.as_ref())
            .map(|(_, type_info)| type_info.clone())
    }

    /// Returns the [`TypeInfo`] referenced by the given [`abi::TypeId`].
    pub fn find_type_info_by_id(&self, type_id: &abi::TypeId) -> Option<Arc<TypeInfo>> {
        self.type_id_to_type_info.get(type_id).map(Clone::clone)
    }

    /// Inserts `type_info` into the type table for a type that has static type info.
    ///
    /// If the type table already contained this `type_info`, the value is updated, and the old
    /// value is returned.
    fn insert_static_type<T: HasStaticTypeInfo + abi::HasStaticTypeId>(
        &mut self,
    ) -> Option<Arc<TypeInfo>> {
        let type_id = T::type_id();
        let type_info = T::type_info();
        self.type_id_to_type_info
            .insert(type_id.clone(), type_info.clone());
        self.type_name_to_type_info
            .insert(type_info.name.clone(), (type_id.clone(), type_info.clone()))
            .map(|entry| entry.1)
    }

    /// Inserts `type_info` into the type table.
    ///
    /// If the type table already contained this `type_info`, the value is updated, and the old
    /// value is returned.
    pub fn insert_type(
        &mut self,
        type_id: &abi::TypeId,
        type_info: Arc<TypeInfo>,
    ) -> Option<Arc<TypeInfo>> {
        self.type_id_to_type_info
            .insert(type_id.clone(), type_info.clone());
        self.type_name_to_type_info
            .insert(type_info.name.clone(), (type_id.clone(), type_info))
            .map(|entry| entry.1)
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
        if let Some((type_id, _)) = &type_info {
            self.type_id_to_type_info.remove(&type_id);
        }
        type_info.map(|(_, type_info)| type_info)
    }
}

impl Default for TypeTable {
    fn default() -> Self {
        let mut type_table = Self {
            type_id_to_type_info: Default::default(),
            type_name_to_type_info: Default::default(),
        };

        type_table.insert_static_type::<i8>();
        type_table.insert_static_type::<i16>();
        type_table.insert_static_type::<i32>();
        type_table.insert_static_type::<i64>();
        type_table.insert_static_type::<i128>();
        type_table.insert_static_type::<u8>();
        type_table.insert_static_type::<u16>();
        type_table.insert_static_type::<u32>();
        type_table.insert_static_type::<u64>();
        type_table.insert_static_type::<u128>();
        type_table.insert_static_type::<f32>();
        type_table.insert_static_type::<f64>();
        type_table.insert_static_type::<bool>();
        type_table.insert_static_type::<()>();
        type_table.insert_static_type::<std::ffi::c_void>();

        // Types used by the FFI interface
        // type_table.insert_type(<*const std::ffi::c_void>::type_info().clone());
        // type_table.insert_type(<*mut std::ffi::c_void>::type_info().clone());
        // type_table.insert_type(<*const *mut std::ffi::c_void>::type_info().clone());

        type_table
    }
}

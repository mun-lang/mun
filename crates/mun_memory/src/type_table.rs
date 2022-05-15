use crate::type_info::TypeInfo;
use abi::HasStaticTypeInfo;
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

        fn insert_primitive_type<T: HasStaticTypeInfo>(type_table: &mut TypeTable) {
            type_table.insert_type(Arc::new(
                TypeInfo::try_from_abi(<T>::type_info(), &type_table).unwrap(),
            ));
        }

        insert_primitive_type::<i8>(&mut type_table);
        insert_primitive_type::<i16>(&mut type_table);
        insert_primitive_type::<i32>(&mut type_table);
        insert_primitive_type::<i64>(&mut type_table);
        insert_primitive_type::<i128>(&mut type_table);
        insert_primitive_type::<u8>(&mut type_table);
        insert_primitive_type::<u16>(&mut type_table);
        insert_primitive_type::<u32>(&mut type_table);
        insert_primitive_type::<u64>(&mut type_table);
        insert_primitive_type::<u128>(&mut type_table);
        insert_primitive_type::<f32>(&mut type_table);
        insert_primitive_type::<f64>(&mut type_table);
        insert_primitive_type::<bool>(&mut type_table);
        insert_primitive_type::<()>(&mut type_table);

        type_table
    }
}

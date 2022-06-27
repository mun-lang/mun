use std::sync::Arc;

use rustc_hash::FxHashMap;

use abi::{Guid, TypeId};

use crate::type_info::{HasStaticTypeInfo, TypeInfo};

#[derive(Clone)]
pub struct TypeTable {
    concrete: FxHashMap<Guid, Arc<TypeInfo>>,
    type_name_to_type_info: FxHashMap<String, Arc<TypeInfo>>,
}

impl TypeTable {
    /// Returns the TypeInfo for the type with the given name.
    pub fn find_type_info_by_name<S: AsRef<str>>(&self, name: S) -> Option<Arc<TypeInfo>> {
        self.type_name_to_type_info.get(name.as_ref()).cloned()
    }

    /// Returns the [`TypeInfo`] referenced by the given [`abi::TypeId`].
    pub fn find_type_info_by_id<'abi>(
        &self,
        type_id: &'abi abi::TypeId<'abi>,
    ) -> Option<Arc<TypeInfo>> {
        match type_id {
            TypeId::Concrete(guid) => self.concrete.get(guid).cloned(),
            TypeId::Pointer(p) => self
                .find_type_info_by_id(p.pointee)
                .map(|ty| ty.pointer_type(p.mutable)),
        }
    }

    /// Inserts `type_info` into the type table for a type that has static type info.
    ///
    /// If the type table already contained this `type_info`, the value is updated, and the old
    /// value is returned.
    fn insert_static_type<T: HasStaticTypeInfo>(&mut self) -> Option<Arc<TypeInfo>> {
        self.insert_type(T::type_info().clone())
    }

    /// Inserts `type_info` into the type table.
    ///
    /// If the type table already contained this `type_info`, the value is updated, and the old
    /// value is returned.
    pub fn insert_type(&mut self, type_info: Arc<TypeInfo>) -> Option<Arc<TypeInfo>> {
        match type_info.as_concrete() {
            None => panic!("can only insert concrete types"),
            Some(guid) => {
                self.type_name_to_type_info
                    .insert(type_info.name.clone(), type_info.clone());
                self.concrete.insert(guid.clone(), type_info)
            }
        }
    }

    /// Removes the specified TypeInfo from the lookup table.
    pub fn remove_type(&mut self, ty: &TypeInfo) -> Option<Arc<TypeInfo>> {
        match ty.as_concrete() {
            None => panic!("can only remove concrete types"),
            Some(guid) => {
                self.type_name_to_type_info.remove(&ty.name);
                self.concrete.remove(guid)
            }
        }
    }

    /// Removes a type described by the given [`abi::TypeInfo`]. Returns `None` if this instance
    /// doesn't hold any type that matches `type_info`.
    pub fn remove_type_by_type_info<'abi>(
        &mut self,
        type_info: &'abi abi::TypeInfo<'abi>,
    ) -> Option<Arc<TypeInfo>> {
        match type_info.as_concrete() {
            None => panic!("can only remove concrete types"),
            Some(guid) => {
                let ty = self.concrete.remove(guid)?;
                self.type_name_to_type_info.remove(&ty.name)
            }
        }
    }

    /// Removes and returns the `TypeInfo` corresponding to `name`, if it exists.
    pub fn remove_type_by_name<S: AsRef<str>>(&mut self, name: S) -> Option<Arc<TypeInfo>> {
        if let Some(type_info) = self.type_name_to_type_info.remove(name.as_ref()) {
            self.remove_type(&type_info)
        } else {
            None
        }
    }
}

impl Default for TypeTable {
    fn default() -> Self {
        let mut type_table = Self {
            concrete: Default::default(),
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

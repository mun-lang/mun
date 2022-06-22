use std::sync::Arc;

use once_cell::sync::OnceCell;
use rustc_hash::FxHashMap;

use abi::{Guid, TypeId};

use crate::type_info::{HasStaticTypeInfo, TypeInfo};
use crate::TypeInfoData;

#[derive(Clone, Default)]
struct TypeLookupNode {
    concrete: FxHashMap<Guid, Arc<TypeInfo>>,
    non_mutable_pointers: OnceCell<Box<TypeLookupNode>>,
    mutable_pointers: OnceCell<Box<TypeLookupNode>>,
}

#[derive(Clone)]
pub struct TypeTable {
    root_node: TypeLookupNode,
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
        fn find_type_info_in_node<'abi>(
            node: &TypeLookupNode,
            type_id: &'abi abi::TypeId<'abi>,
        ) -> Option<Arc<TypeInfo>> {
            match type_id {
                abi::TypeId::Concrete(guid) => node.concrete.get(guid).cloned(),
                abi::TypeId::Pointer(p) => {
                    let node = if p.mutable {
                        node.mutable_pointers.get()
                    } else {
                        node.non_mutable_pointers.get()
                    };
                    node.and_then(|node| find_type_info_in_node(node.as_ref(), p.pointee))
                }
            }
        }
        find_type_info_in_node(&self.root_node, type_id)
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
        fn insert_type_info_in_node(
            node: &mut TypeLookupNode,
            type_info: &Arc<TypeInfo>,
            original_type_info: &Arc<TypeInfo>,
        ) {
            match &type_info.data {
                TypeInfoData::Primitive(guid) => {
                    node.concrete
                        .insert(guid.clone(), original_type_info.clone());
                }
                TypeInfoData::Struct(s) => {
                    node.concrete
                        .insert(s.guid.clone(), original_type_info.clone());
                }
                TypeInfoData::Pointer(p) => {
                    let node = if p.mutable {
                        &mut node.mutable_pointers
                    } else {
                        &mut node.non_mutable_pointers
                    };
                    node.get_or_init(Default::default);
                    insert_type_info_in_node(
                        node.get_mut().expect("initialization failed"),
                        &p.pointee,
                        original_type_info,
                    )
                }
            }
        }
        insert_type_info_in_node(&mut self.root_node, &type_info, &type_info);
        self.type_name_to_type_info
            .insert(type_info.name.clone(), type_info)
    }

    /// Removes the specified TypeInfo from the lookup table.
    pub fn remove_type(&mut self, ty: &TypeInfo) -> Option<Arc<TypeInfo>> {
        fn remove_type_from_node(
            node: &mut TypeLookupNode,
            ty: &TypeInfo,
        ) -> Option<Arc<TypeInfo>> {
            match &ty.data {
                TypeInfoData::Primitive(guid) => node.concrete.remove(guid),
                TypeInfoData::Struct(s) => node.concrete.remove(&s.guid),
                TypeInfoData::Pointer(p) => {
                    if let Some(node) = if p.mutable {
                        node.mutable_pointers.get_mut()
                    } else {
                        node.non_mutable_pointers.get_mut()
                    } {
                        remove_type_from_node(node, &p.pointee)
                    } else {
                        None
                    }
                }
            }
        }
        remove_type_from_node(&mut self.root_node, ty)
    }

    /// Removes a type described by the given [`abi::TypeInfo`]. Returns `None` if this instance
    /// doesn't hold any type that matches `type_info`.
    pub fn remove_type_by_type_info<'abi>(&mut self, type_info: &'abi abi::TypeInfo<'abi>) -> Option<Arc<TypeInfo>> {
        let (mut node, mut type_id) = match &type_info.data {
            abi::TypeInfoData::Primitive(guid) => return self.root_node.concrete.remove(guid),
            abi::TypeInfoData::Struct(s) => return self.root_node.concrete.remove(&s.guid),
            abi::TypeInfoData::Pointer(p) => {
                (if p.mutable {
                    self.root_node.mutable_pointers.get_mut()?
                } else {
                    self.root_node.non_mutable_pointers.get_mut()?
                }, &p.pointee)
            }
        };

        loop {
            match type_id {
                TypeId::Concrete(guid) => {
                    return node.concrete.remove(guid);
                },
                TypeId::Pointer(p) => {
                    node = if p.mutable {
                        node.mutable_pointers.get_mut()?
                    } else {
                        node.non_mutable_pointers.get_mut()?
                    };
                    type_id = p.pointee;
                }
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
            root_node: Default::default(),
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

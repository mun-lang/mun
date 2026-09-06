use std::{cell::RefCell, sync::Arc};

use rustc_hash::FxHashMap;

use crate::{
    ir::types::{AbiBuilder, TypeIdGlobal, TypeIdValue},
    type_info::{TypeId, TypeIdData},
};

/// Builds runtime type identifiers and interns identifiers referenced by other
/// identifiers.
pub struct TypeIdBuilder<'a, 'ctx, 'ink> {
    abi: &'a AbiBuilder<'ctx, 'ink>,
    interned_types: RefCell<FxHashMap<Arc<TypeId>, TypeIdGlobal<'ink>>>,
}

impl<'a, 'ctx, 'ink> TypeIdBuilder<'a, 'ctx, 'ink> {
    pub fn new(abi: &'a AbiBuilder<'ctx, 'ink>) -> Self {
        Self {
            abi,
            interned_types: RefCell::new(FxHashMap::default()),
        }
    }

    /// Converts the compiler-owned identifier to its concrete runtime ABI
    /// representation.
    pub fn construct_from_type_id(&self, type_id: &Arc<TypeId>) -> TypeIdValue<'ink> {
        match &type_id.data {
            TypeIdData::Concrete(guid) => self.abi.types.concrete_type_id(*guid),
            TypeIdData::Pointer(pointer) => self
                .abi
                .types
                .pointer_type_id(self.get_global_type_id(&pointer.pointee), pointer.mutable),
            TypeIdData::Array(element) => self
                .abi
                .types
                .array_type_id(self.get_global_type_id(element)),
        }
    }

    /// Returns a stable global address for an identifier referenced by another
    /// identifier.
    fn get_global_type_id(&self, type_id: &Arc<TypeId>) -> TypeIdGlobal<'ink> {
        if let Some(value) = self.interned_types.borrow().get(type_id).copied() {
            return value;
        }

        let value = self.construct_from_type_id(type_id);
        let global = self
            .abi
            .types
            .private_type_id_global(self.abi.module, &type_id.name, value);
        self.interned_types
            .borrow_mut()
            .insert(type_id.clone(), global);
        global
    }
}

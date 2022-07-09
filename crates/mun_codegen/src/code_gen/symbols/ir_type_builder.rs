use std::{cell::RefCell, sync::Arc};

use inkwell::{module::Linkage, values::UnnamedAddress};
use rustc_hash::FxHashMap;

use crate::{
    ir::types as ir,
    type_info::{TypeId, TypeIdData},
    value::{AsValue, Global, IrValueContext},
};

/// An object that constructs [`ir::TypeId`]s from various representations.
///
/// This object also caches any types that are referenced by other `TypeId`s. Types that reference
/// other types are for instance pointers or arrays.
pub struct TypeIdBuilder<'ink, 'a, 'b, 'c> {
    context: &'a IrValueContext<'ink, 'b, 'c>,

    /// A map of ir::TypeIds that have already have already been interned.
    interned_types: RefCell<FxHashMap<Arc<TypeId>, Global<'ink, ir::TypeId<'ink>>>>,
}

impl<'ink, 'a, 'b, 'c> TypeIdBuilder<'ink, 'a, 'b, 'c> {
    pub fn new(context: &'a IrValueContext<'ink, 'b, 'c>) -> Self {
        Self {
            context,
            interned_types: RefCell::new(Default::default()),
        }
    }

    /// Constructs an [`ir::TypeId`] from an internal TypeId.
    pub fn construct_from_type_id(&self, type_id: &Arc<TypeId>) -> ir::TypeId<'ink> {
        match &type_id.data {
            TypeIdData::Concrete(guid) => ir::TypeId::Concrete(*guid),
            TypeIdData::Pointer(p) => {
                let pointee = self.get_global_type_id(&p.pointee);
                ir::TypeId::Pointer(ir::PointerTypeId {
                    pointee,
                    mutable: p.mutable,
                })
            }
            TypeIdData::Array(arr) => {
                let element = self.get_global_type_id(arr);
                ir::TypeId::Array(ir::ArrayTypeId { element })
            }
        }
    }

    /// Returns the global pointer to the specific type
    fn get_global_type_id(&self, type_id: &Arc<TypeId>) -> Global<'ink, ir::TypeId<'ink>> {
        let global = match {
            let borrow = self.interned_types.borrow();
            borrow.get(type_id.as_ref()).cloned()
        } {
            Some(v) => v,
            None => {
                let pointee_ir_type_id = self.construct_from_type_id(type_id);
                let global = pointee_ir_type_id.as_value(self.context).into_global(
                    &type_id.name,
                    self.context,
                    true,
                    Linkage::Private,
                    Some(UnnamedAddress::Global),
                );
                self.interned_types
                    .borrow_mut()
                    .insert(type_id.clone(), global);
                global
            }
        };
        global
    }
}

use std::collections::HashMap;
use abi::{HasStaticTypeId, TypeId};
use hir::{FloatBitness, HirDatabase, HirDisplay, Ty, TyKind};
use crate::ir::types as ir;
use crate::value::{Global, IrValueContext};

/// An object that constructs [`ir::TypeId`]s from various representations.
///
/// This object also caches any types that are referenced by other `TypeId`s. Types that reference
/// other types are for instance pointers or arrays.
struct TypeIdBuilder<'ink, 'a> {
    context: &'a IrValueContext<'ink, '_, '_>,

    /// A map of abi::TypeIds that have already have a global associated with it.
    global_types: HashMap<abi::TypeId, Global<'ink, ir::TypeId<'ink>>>
}

impl TypeIdBuilder {
    /// Constructs an [`ir::TypeId`] from a HIR type
    pub fn construct_from_hir(&self, ty: &hir::Ty, db: &dyn HirDatabase) -> ir::TypeId {
        match ty.interned() {
            TyKind::Struct(_) => {}
            TyKind::Float(f) => self.construct_from_hir_float(f, db),
            TyKind::Int(_) => {}
            TyKind::Bool => {}
            TyKind::Tuple(_, _) => {}
            _ => unimplemented!("{} unhandled", ty.display(db)),
        }
    }

    /// Constructs a [`ir::TypeId`] from a HIR float type
    pub fn construct_from_hir_float(&self, ty: &hir::FloatTy, db: &dyn HirDatabase) -> ir::TypeId {
        match ty.bitness {
            FloatBitness::X32 => self.construct_from_abi(f32::type_id()),
            FloatBitness::X64 => self.construct_from_abi(f64::type_id()),
        }
    }

    pub fn construct_from_abi<'abi>(&self, ty: &'abi abi::TypeId<'abi>) -> ir::TypeId {
        match ty {
            TypeId::Concrete(guid) => ir::TypeId::Concrete(guid.clone()),
            TypeId::Pointer(p) => p.
        }
    }
}

use crate::{
    item_tree::{Function, ItemTreeId, ItemTreeNode, Struct, TypeAlias},
    module_tree::LocalModuleId,
    primitive_type::PrimitiveType,
    DefDatabase, PackageId,
};
use std::hash::{Hash, Hasher};

#[derive(Debug)]
pub struct ItemLoc<N: ItemTreeNode> {
    pub id: ItemTreeId<N>,
}

impl<N: ItemTreeNode> PartialEq for ItemLoc<N> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<N: ItemTreeNode> Eq for ItemLoc<N> {}

impl<N: ItemTreeNode> Hash for ItemLoc<N> {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.id.hash(hasher);
    }
}

impl<N: ItemTreeNode> Clone for ItemLoc<N> {
    fn clone(&self) -> ItemLoc<N> {
        ItemLoc { id: self.id }
    }
}
impl<N: ItemTreeNode> Copy for ItemLoc<N> {}

#[derive(Debug)]
pub struct AssocItemLoc<N: ItemTreeNode> {
    pub module: ModuleId,
    pub id: ItemTreeId<N>,
}

impl<N: ItemTreeNode> Clone for AssocItemLoc<N> {
    fn clone(&self) -> Self {
        Self {
            module: self.module,
            id: self.id,
        }
    }
}

impl<N: ItemTreeNode> Copy for AssocItemLoc<N> {}

impl<N: ItemTreeNode> PartialEq for AssocItemLoc<N> {
    fn eq(&self, other: &Self) -> bool {
        self.module == other.module && self.id == other.id
    }
}

impl<N: ItemTreeNode> Eq for AssocItemLoc<N> {}

impl<N: ItemTreeNode> Hash for AssocItemLoc<N> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.module.hash(state);
        self.id.hash(state);
    }
}

macro_rules! impl_intern_key {
    ($name:ident) => {
        impl salsa::InternKey for $name {
            fn from_intern_id(v: salsa::InternId) -> Self {
                $name(v)
            }
            fn as_intern_id(&self) -> salsa::InternId {
                self.0
            }
        }
    };
}

macro_rules! impl_intern {
    ($id:ident, $loc:ident, $intern:ident, $lookup:ident) => {
        impl_intern_key!($id);

        impl Intern for $loc {
            type ID = $id;
            fn intern(self, db: &dyn DefDatabase) -> $id {
                db.$intern(self)
            }
        }

        impl Lookup for $id {
            type Data = $loc;
            fn lookup(&self, db: &dyn DefDatabase) -> $loc {
                db.$lookup(*self)
            }
        }
    };
}

/// Represents an id of a module inside a package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct ModuleId {
    pub package: PackageId,
    pub local_id: LocalModuleId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct FunctionId(salsa::InternId);
pub(crate) type FunctionLoc = AssocItemLoc<Function>;
impl_intern!(
    FunctionId,
    FunctionLoc,
    intern_function,
    lookup_intern_function
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StructId(salsa::InternId);
pub(crate) type StructLoc = AssocItemLoc<Struct>;
impl_intern!(StructId, StructLoc, intern_struct, lookup_intern_struct);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeAliasId(salsa::InternId);
pub(crate) type TypeAliasLoc = AssocItemLoc<TypeAlias>;
impl_intern!(
    TypeAliasId,
    TypeAliasLoc,
    intern_type_alias,
    lookup_intern_type_alias
);

pub trait Intern {
    type ID;
    fn intern(self, db: &dyn DefDatabase) -> Self::ID;
}

pub trait Lookup {
    type Data;
    fn lookup(&self, db: &dyn DefDatabase) -> Self::Data;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemDefinitionId {
    ModuleId(ModuleId),
    FunctionId(FunctionId),
    StructId(StructId),
    TypeAliasId(TypeAliasId),
    PrimitiveType(PrimitiveType),
}

impl From<ModuleId> for ItemDefinitionId {
    fn from(id: ModuleId) -> Self {
        ItemDefinitionId::ModuleId(id)
    }
}
impl From<FunctionId> for ItemDefinitionId {
    fn from(id: FunctionId) -> Self {
        ItemDefinitionId::FunctionId(id)
    }
}
impl From<StructId> for ItemDefinitionId {
    fn from(id: StructId) -> Self {
        ItemDefinitionId::StructId(id)
    }
}
impl From<TypeAliasId> for ItemDefinitionId {
    fn from(id: TypeAliasId) -> Self {
        ItemDefinitionId::TypeAliasId(id)
    }
}
impl From<PrimitiveType> for ItemDefinitionId {
    fn from(id: PrimitiveType) -> Self {
        ItemDefinitionId::PrimitiveType(id)
    }
}

/// Definitions which have a body
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefWithBodyId {
    FunctionId(FunctionId),
}

impl From<FunctionId> for DefWithBodyId {
    fn from(id: FunctionId) -> Self {
        DefWithBodyId::FunctionId(id)
    }
}

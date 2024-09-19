use std::hash::{Hash, Hasher};

use crate::{
    item_tree::{Enum, Function, Impl, ItemTreeId, ItemTreeNode, Struct, TypeAlias, Variant},
    module_tree::LocalModuleId,
    primitive_type::PrimitiveType,
    DefDatabase, PackageId,
};

#[derive(Clone, Debug)]
pub struct ItemLoc<N: ItemTreeNode> {
    pub module: ModuleId,
    pub id: ItemTreeId<N>,
}

impl<N: ItemTreeNode> PartialEq for ItemLoc<N> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.module == other.module
    }
}

impl<N: ItemTreeNode> Eq for ItemLoc<N> {}

impl<N: ItemTreeNode> Hash for ItemLoc<N> {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.module.hash(hasher);
        self.id.hash(hasher);
    }
}

impl<N: ItemTreeNode> Copy for ItemLoc<N> {}

#[derive(Clone, Debug)]
pub struct AssocItemLoc<N: ItemTreeNode> {
    pub container: ItemContainerId,
    pub id: ItemTreeId<N>,
}

impl<N: ItemTreeNode> Copy for AssocItemLoc<N> {}

impl<N: ItemTreeNode> PartialEq for AssocItemLoc<N> {
    fn eq(&self, other: &Self) -> bool {
        self.container == other.container && self.id == other.id
    }
}

impl<N: ItemTreeNode> Eq for AssocItemLoc<N> {}

impl<N: ItemTreeNode> Hash for AssocItemLoc<N> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.container.hash(state);
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

/// Represents an id of an item inside a item container such as a module or a
/// `impl` block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemContainerId {
    ModuleId(ModuleId),
    ImplId(ImplId),
}

impl_froms!(ModuleId, ImplId for ItemContainerId);

/// An abstract data type ID
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AdtId {
    EnumId(EnumId),
    StructId(StructId),
}

impl_froms!(EnumId, StructId for AdtId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct ImplId(salsa::InternId);

pub(crate) type ImplLoc = ItemLoc<Impl>;
impl_intern!(ImplId, ImplLoc, intern_impl, lookup_intern_impl);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct EnumId(salsa::InternId);

pub(crate) type EnumLoc = ItemLoc<Enum>;
impl_intern!(EnumId, EnumLoc, intern_enum, lookup_intern_enum);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EnumVariantId(salsa::InternId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EnumVariantLoc {
    pub id: ItemTreeId<Variant>,
    pub parent: EnumId,
    pub index: u32,
}

impl_intern!(
    EnumVariantId,
    EnumVariantLoc,
    intern_enum_variant,
    lookup_intern_enum_variant
);

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

pub(crate) type StructLoc = ItemLoc<Struct>;
impl_intern!(StructId, StructLoc, intern_struct, lookup_intern_struct);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeAliasId(salsa::InternId);

pub(crate) type TypeAliasLoc = ItemLoc<TypeAlias>;
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
    AdtId(AdtId),
    TypeAliasId(TypeAliasId),
    PrimitiveType(PrimitiveType),
}

impl_froms!(
    ModuleId,
    FunctionId,
    AdtId(EnumId, StructId),
    TypeAliasId,
    PrimitiveType for ItemDefinitionId
);

/// Items that are associated with an `impl`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum AssocItemId {
    FunctionId(FunctionId),
}

/// Definitions which have a body
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefWithBodyId {
    FunctionId(FunctionId),
}

impl_froms!(FunctionId for DefWithBodyId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldOwnerId {
    EnumVariantId(EnumVariantId),
    StructId(StructId),
}

impl_froms!(EnumVariantId, StructId for FieldOwnerId);

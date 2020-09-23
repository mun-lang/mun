use crate::item_tree::{Function, ItemTreeId, ItemTreeNode, Struct, TypeAlias};
use crate::DefDatabase;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct FunctionId(salsa::InternId);
pub(crate) type FunctionLoc = ItemLoc<Function>;
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

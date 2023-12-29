use crate::{
    ids::{AssocItemLoc, FunctionId, ImplId, ItemContainerId, Lookup, StructId, TypeAliasId},
    item_tree::ItemTreeNode,
    DefDatabase, ModuleId,
};

/// A trait to lookup the module associated with an item.
pub trait HasModule {
    fn module(&self, db: &dyn DefDatabase) -> ModuleId;
}

impl HasModule for ItemContainerId {
    fn module(&self, db: &dyn DefDatabase) -> ModuleId {
        match self {
            ItemContainerId::ModuleId(it) => *it,
            ItemContainerId::ImplId(it) => it.lookup(db).module,
        }
    }
}

impl<N: ItemTreeNode> HasModule for AssocItemLoc<N> {
    fn module(&self, db: &dyn DefDatabase) -> ModuleId {
        self.container.module(db)
    }
}

impl HasModule for StructId {
    fn module(&self, db: &dyn DefDatabase) -> ModuleId {
        self.lookup(db).module
    }
}

impl HasModule for FunctionId {
    fn module(&self, db: &dyn DefDatabase) -> ModuleId {
        self.lookup(db).container.module(db)
    }
}

impl HasModule for ImplId {
    fn module(&self, db: &dyn DefDatabase) -> ModuleId {
        self.lookup(db).module
    }
}

impl HasModule for TypeAliasId {
    fn module(&self, db: &dyn DefDatabase) -> ModuleId {
        self.lookup(db).module
    }
}

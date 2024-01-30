use std::sync::Arc;

use crate::{
    has_module::HasModule,
    ids::{AssocItemId, FunctionLoc, ImplId, Intern, ItemContainerId, Lookup},
    item_tree::{AssociatedItem, ItemTreeId},
    type_ref::{LocalTypeRefId, TypeRefMap, TypeRefMapBuilder, TypeRefSourceMap},
    DefDatabase, FileId, Function, HirDatabase, ItemLoc, Module, Package, Ty,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Impl {
    pub(crate) id: ImplId,
}

impl Impl {
    /// Returns all the implementations defined in the specified `package`.
    pub fn all_in_package(db: &dyn HirDatabase, package: Package) -> Vec<Impl> {
        let inherent_impls = db.inherent_impls_in_package(package.id);
        inherent_impls.all_impls().map(Self::from).collect()
    }

    /// The module in which the `impl` was defined.
    ///
    /// Note that this is not necessarily the module in which the self type was
    /// defined. `impl`s can be defined in any module from where the self
    /// type is visibile.
    pub fn module(self, db: &dyn HirDatabase) -> Module {
        self.id.module(db.upcast()).into()
    }

    /// Returns the file in which the implementation was defined
    pub fn file_id(self, db: &dyn HirDatabase) -> FileId {
        self.id.lookup(db.upcast()).id.file_id
    }

    /// Returns the type for which this is an implementation
    pub fn self_ty(self, db: &dyn HirDatabase) -> Ty {
        let data = db.impl_data(self.id);
        let lowered = db.lower_impl(self.id);
        lowered[data.self_ty].clone()
    }

    /// Returns all the items in the implementation
    pub fn items(self, db: &dyn HirDatabase) -> Vec<AssocItem> {
        db.impl_data(self.id)
            .items
            .iter()
            .copied()
            .map(Into::into)
            .collect()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum AssocItem {
    Function(Function),
}

impl From<AssocItemId> for AssocItem {
    fn from(value: AssocItemId) -> Self {
        match value {
            AssocItemId::FunctionId(it) => AssocItem::Function(it.into()),
        }
    }
}

impl From<ImplId> for Impl {
    fn from(value: ImplId) -> Self {
        Impl { id: value }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ImplData {
    pub items: Vec<AssocItemId>,
    pub self_ty: LocalTypeRefId,
    pub type_ref_map: TypeRefMap,
    pub type_ref_source_map: TypeRefSourceMap,
}

impl ImplData {
    pub(crate) fn impl_data_query(db: &dyn DefDatabase, id: ImplId) -> Arc<ImplData> {
        let ItemLoc {
            module: _,
            id: tree_id,
        } = id.lookup(db);

        let item_tree = db.item_tree(tree_id.file_id);
        let impl_def = &item_tree[tree_id.value];
        let src = item_tree.source(db, tree_id.value);

        // Associate the self type
        let mut type_builder = TypeRefMapBuilder::default();
        let self_ty = type_builder.alloc_from_node_opt(src.type_ref().as_ref());
        let (type_ref_map, type_ref_source_map) = type_builder.finish();

        // Add all the associated items
        let container = ItemContainerId::ImplId(id);
        let items = impl_def
            .items
            .iter()
            .map(|it| match it {
                AssociatedItem::Function(id) => {
                    let loc = FunctionLoc {
                        container,
                        id: ItemTreeId::new(tree_id.file_id, *id),
                    };
                    let func_id = loc.intern(db);
                    AssocItemId::FunctionId(func_id)
                }
            })
            .collect();

        Arc::new(ImplData {
            items,
            self_ty,
            type_ref_map,
            type_ref_source_map,
        })
    }
}

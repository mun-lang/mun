use std::{iter::successors, sync::Arc};

use la_arena::ArenaMap;
use mun_hir_input::{ModuleId, ModuleTree, PackageModuleId};
use mun_syntax::ast;

use crate::{
    code_model::r#struct::LocalFieldId,
    has_module::HasModule,
    ids::{FunctionId, VariantId},
    resolve::HasResolver,
    DefDatabase, HirDatabase, Module, Resolver,
};

/// Visibility of an item, not yet resolved to an actual module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawVisibility {
    /// Accessible from self (Self) and all sub-modules
    This,

    /// Accessible from self, parent module, and all sub-modules
    Super,

    /// Accessible from everywhere in the package
    Package,

    /// Accessible from everywhere
    Public,
}

impl RawVisibility {
    /// Constructs a private visibility only visible from `self`.
    const fn private() -> RawVisibility {
        RawVisibility::This
    }

    /// Constructs a `RawVisibility` from an AST node.
    pub(crate) fn from_ast(node: Option<ast::Visibility>) -> RawVisibility {
        let node = match node {
            None => return RawVisibility::private(),
            Some(node) => node,
        };

        match node.kind() {
            ast::VisibilityKind::Pub => RawVisibility::Public,
            ast::VisibilityKind::PubSuper => RawVisibility::Super,
            ast::VisibilityKind::PubPackage => RawVisibility::Package,
        }
    }

    pub fn resolve(&self, db: &dyn DefDatabase, resolver: &Resolver) -> Visibility {
        // we fall back to public visibility (i.e. fail open) if the path can't be
        // resolved
        resolver
            .resolve_visibility(db, self)
            .unwrap_or(Visibility::Public)
    }
}

/// Visibility of an item, modules resolved.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Visibility {
    /// Accessible from the specified module and all sub-modules
    Module(ModuleId),

    /// Publicly accessible
    Public,
}

impl Visibility {
    /// Returns true if an item with this visibility is accessible from the
    /// module of the specified `PackageDefs`.
    pub(crate) fn is_visible_from_module_tree(
        self,
        module_tree: &ModuleTree,
        from_module: PackageModuleId,
    ) -> bool {
        let to_module = match self {
            Visibility::Module(m) => m,
            Visibility::Public => return true,
        };

        let mut ancestors = successors(Some(from_module), |m| module_tree[*m].parent);

        ancestors.any(|m| m == to_module.local_id)
    }

    /// Returns true if an item with this visibility is accessible from the
    /// given module.
    pub fn is_visible_from(self, db: &dyn HirDatabase, from_module: ModuleId) -> bool {
        let to_module = match self {
            Visibility::Module(m) => m,
            Visibility::Public => return true,
        };

        let module_tree = db.module_tree(from_module.package);
        let mut ancestors = successors(Some(from_module.local_id), |m| module_tree[*m].parent);

        ancestors.any(|m| m == to_module.local_id)
    }

    /// Returns true if an item with this visibility is accessible externally
    pub fn is_externally_visible(self) -> bool {
        match self {
            Visibility::Module(_) => false,
            Visibility::Public => true,
        }
    }

    /// Converts a `RawVisibility` which describes the visibility of an item
    /// relative to a module into a `Visibility` which describes the
    /// absolute visibility within the module tree.
    pub(crate) fn resolve(
        _db: &dyn DefDatabase,
        module_tree: &ModuleTree,
        original_module: PackageModuleId,
        visibility: &RawVisibility,
    ) -> Visibility {
        match visibility {
            RawVisibility::This => Visibility::Module(ModuleId {
                package: module_tree.package,
                local_id: original_module,
            }),
            RawVisibility::Super => {
                let parent_module_id = module_tree[original_module]
                    .parent
                    .unwrap_or(original_module);
                Visibility::Module(ModuleId {
                    package: module_tree.package,
                    local_id: parent_module_id,
                })
            }
            RawVisibility::Package => Visibility::Module(ModuleId {
                package: module_tree.package,
                local_id: module_tree.root,
            }),
            RawVisibility::Public => Visibility::Public,
        }
    }
}

pub trait HasVisibility {
    /// Returns the visibility of the item.
    fn visibility(&self, db: &dyn HirDatabase) -> Visibility;

    /// Returns true if the item is visible from the specified module.
    fn is_visible_from(&self, db: &dyn HirDatabase, module: Module) -> bool {
        let vis = self.visibility(db);
        vis.is_visible_from(db, module.id)
    }
}

/// Resolve visibility of a function.
pub(crate) fn function_visibility_query(db: &dyn DefDatabase, def: FunctionId) -> Visibility {
    let resolver = def.resolver(db);
    db.fn_data(def).visibility().resolve(db, &resolver)
}

/// Resolve visibility of all fields of a variant.
pub(crate) fn field_visibilities_query(
    db: &dyn DefDatabase,
    variant_id: VariantId,
) -> Arc<ArenaMap<LocalFieldId, Visibility>> {
    let mut res = ArenaMap::default();
    let resolver = variant_id.module(db).resolver(db);
    match variant_id {
        VariantId::StructId(st) => {
            let struct_data = db.struct_data(st);
            for (field_idx, field_data) in struct_data.fields.iter() {
                res.insert(field_idx, field_data.visibility.resolve(db, &resolver));
            }
        }
    };
    Arc::new(res)
}

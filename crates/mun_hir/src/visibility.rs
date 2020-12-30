use crate::module_tree::{LocalModuleId, ModuleTree};
use crate::{ids::ModuleId, DefDatabase, HirDatabase, Resolver};
use mun_syntax::ast;
use std::iter::successors;

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
        // we fall back to public visibility (i.e. fail open) if the path can't be resolved
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
    /// Returns true if an item with this visibility is accessible from the module from the
    /// specified `PackageDefs`.
    pub(crate) fn is_visible_from_module_tree(
        self,
        module_tree: &ModuleTree,
        from_module: LocalModuleId,
    ) -> bool {
        let to_module = match self {
            Visibility::Module(m) => m,
            Visibility::Public => return true,
        };

        let mut ancestors = successors(Some(from_module), |m| module_tree[*m].parent);

        ancestors.any(|m| m == to_module.local_id)
    }

    /// Returns true if an item with this visibility is accessible from the given module.
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
}

pub trait HasVisibility {
    fn visibility(&self, db: &dyn HirDatabase) -> Visibility;
}

use super::Module;
use crate::{HirDatabase, ModuleId, PackageId};

/// A `Package` describes a single package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Package {
    pub(crate) id: PackageId,
}

impl Package {
    /// Returns all the packages defined in the database
    pub fn all(db: &dyn HirDatabase) -> Vec<Package> {
        db.packages().iter().map(|id| Package { id }).collect()
    }

    /// Returns the root module of the package (represented by the `mod.rs` in the source root)
    pub fn root_module(self, db: &dyn HirDatabase) -> Module {
        let module_tree = db.module_tree(self.id);
        Module {
            id: ModuleId {
                package: self.id,
                local_id: module_tree.root,
            },
        }
    }

    /// Returns all the modules in the package
    pub fn modules(self, db: &dyn HirDatabase) -> Vec<Module> {
        let module_tree = db.module_tree(self.id);
        module_tree
            .modules
            .iter()
            .map(|(idx, _)| Module {
                id: ModuleId {
                    package: self.id,
                    local_id: idx,
                },
            })
            .collect()
    }
}

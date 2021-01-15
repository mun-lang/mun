mod collector;

use crate::{
    arena::map::ArenaMap, item_scope::ItemScope, module_tree::LocalModuleId,
    module_tree::ModuleTree, DefDatabase, PackageId,
};
use std::{ops::Index, sync::Arc};

/// Contains all top-level definitions for a package.
#[derive(Debug, PartialEq, Eq)]
pub struct PackageDefs {
    pub modules: ArenaMap<LocalModuleId, ItemScope>,
    pub module_tree: Arc<ModuleTree>,
}

impl PackageDefs {
    pub(crate) fn package_def_map_query(
        db: &dyn DefDatabase,
        package: PackageId,
    ) -> Arc<PackageDefs> {
        Arc::new(collector::collect(db, package))
    }
}

impl Index<LocalModuleId> for PackageDefs {
    type Output = ItemScope;

    fn index(&self, index: LocalModuleId) -> &Self::Output {
        &self.modules[index]
    }
}

use crate::{CodeGenDatabase, ModuleGroup};
use rustc_hash::FxHashMap;
use std::ops::Index;
use std::sync::Arc;

/// A `ModuleGroupId` refers to a single [`ModuleGroup`] in a [`ModulePartition`]
#[derive(Default, PartialEq, Eq, Clone, Debug, Hash, PartialOrd, Ord, Copy)]
pub struct ModuleGroupId(usize);

/// A `ModulePartition` defines how modules are grouped together.
#[derive(Default, PartialEq, Eq, Clone, Debug)]
pub struct ModulePartition {
    groups: Vec<ModuleGroup>,
    module_to_group: FxHashMap<mun_hir::Module, ModuleGroupId>,
    file_to_group: FxHashMap<mun_hir::FileId, ModuleGroupId>,
}

impl ModulePartition {
    /// Adds a new group of modules to the partition. This function panics if a module is added
    /// twice in different groups.
    pub fn add_group(
        &mut self,
        db: &dyn mun_hir::HirDatabase,
        group: ModuleGroup,
    ) -> ModuleGroupId {
        let id = ModuleGroupId(self.groups.len());
        for module in group.iter() {
            assert!(
                self.module_to_group.insert(module, id).is_none(),
                "cannot add a module to multiple groups"
            );
            if let Some(file_id) = module.file_id(db) {
                assert!(
                    self.file_to_group.insert(file_id, id).is_none(),
                    "cannot add a file to multiple groups"
                );
            }
        }

        self.groups.push(group);
        id
    }

    /// Returns the group to which the specified module belongs.
    pub fn group_for_module(&self, module: mun_hir::Module) -> Option<ModuleGroupId> {
        self.module_to_group.get(&module).copied()
    }

    /// Returns the group to which the specified module belongs.
    pub fn group_for_file(&self, file: mun_hir::FileId) -> Option<ModuleGroupId> {
        self.file_to_group.get(&file).copied()
    }

    /// Returns an iterator over all the groups
    pub fn iter(&self) -> impl Iterator<Item = (ModuleGroupId, &ModuleGroup)> + '_ {
        self.groups
            .iter()
            .enumerate()
            .map(|(idx, group)| (ModuleGroupId(idx), group))
    }
}

impl Index<ModuleGroupId> for ModulePartition {
    type Output = ModuleGroup;

    fn index(&self, index: ModuleGroupId) -> &Self::Output {
        &self.groups[index.0]
    }
}

/// Builds a module partition from the contents of the database
pub(crate) fn build_partition(db: &dyn CodeGenDatabase) -> Arc<ModulePartition> {
    let mut partition = ModulePartition::default();
    for module in mun_hir::Package::all(db.upcast())
        .into_iter()
        .flat_map(|package| package.modules(db.upcast()))
    {
        let name = if module.name(db.upcast()).is_some() {
            module.full_name(db.upcast())
        } else {
            String::from("mod")
        };

        partition.add_group(
            db.upcast(),
            ModuleGroup::new(db.upcast(), name, vec![module]),
        );
    }
    Arc::new(partition)
}

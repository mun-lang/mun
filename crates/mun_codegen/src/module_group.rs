//! A `ModuleGroup` describes a grouping of modules that together form an assembly.

use mun_hir::{HasVisibility, HirDatabase};
use rustc_hash::{FxHashMap, FxHashSet};
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;

/// A `ModuleGroup` describes a grouping of modules
#[derive(Clone, Eq, Debug)]
pub struct ModuleGroup {
    ordered_modules: Vec<mun_hir::Module>,
    modules: FxHashSet<mun_hir::Module>,
    includes_entire_subtree: FxHashMap<mun_hir::Module, bool>,
    pub name: String,
}

impl Hash for ModuleGroup {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ordered_modules.hash(state);
        self.name.hash(state);
    }
}

impl PartialEq for ModuleGroup {
    fn eq(&self, other: &Self) -> bool {
        self.ordered_modules == other.ordered_modules && self.name == other.name
    }
}

impl ModuleGroup {
    /// Constructs a new `ModuleGroup` from a collection of modules and a name.
    pub fn new(
        db: &dyn HirDatabase,
        name: String,
        modules: impl IntoIterator<Item = mun_hir::Module>,
    ) -> Self {
        let modules = FxHashSet::from_iter(modules);
        let includes_entire_subtree = modules
            .iter()
            .map(|&module| {
                (
                    module,
                    module
                        .children(db)
                        .iter()
                        .all(|child_module| modules.contains(child_module)),
                )
            })
            .collect();

        let mut ordered_modules: Vec<mun_hir::Module> = modules.iter().copied().collect();
        ordered_modules.sort();

        Self {
            ordered_modules,
            modules,
            includes_entire_subtree,
            name,
        }
    }

    /// Constructs a new module group from a single module
    pub fn from_single_module(db: &dyn HirDatabase, module: mun_hir::Module) -> Self {
        Self::new(db, module.full_name(db), vec![module])
    }

    /// Returns true if the specified `mun_hir::Module` is part of this group.
    pub fn contains(&self, module: mun_hir::Module) -> bool {
        self.modules.contains(&module)
    }

    /// Returns an iterator over all modules in the group
    pub fn iter(&self) -> impl Iterator<Item = mun_hir::Module> + '_ {
        self.ordered_modules.iter().copied()
    }

    /// Returns true if the specified function should be exported from the module group. This
    /// indicates that when queried the resulting assembly will expose this function.
    pub fn should_export_fn(&self, db: &dyn HirDatabase, function: mun_hir::Function) -> bool {
        // If the function is not defined in the module group we should definitely not export it.
        if !self.modules.contains(&function.module(db)) {
            return false;
        }

        let vis = function.visibility(db);
        match vis {
            // If the function is publicly accessible it must always be exported
            mun_hir::Visibility::Public => true,

            // The function is visible from the specified module and all child modules.
            mun_hir::Visibility::Module(visible_mod) => {
                // If the modules is contained within `includes_entire_subtree` it is included in
                // the module group.
                self.includes_entire_subtree
                    .get(&visible_mod.into())
                    // If all its children are also part of the module group we can keep the
                    // function internal, so there is no need to export it.
                    .map_or(true, |&includes_subtree| !includes_subtree)
            }
        }
    }

    /// Returns true if the specified function should be included in the dispatch table of this
    /// module group if it is used from within this module group.
    pub fn should_runtime_link_fn(
        &self,
        db: &dyn HirDatabase,
        function: mun_hir::Function,
    ) -> bool {
        function.is_extern(db) || !self.modules.contains(&function.module(db))
    }

    /// Returns the `mun_hir::FileId`s that are included in this module group.
    pub fn files<'s>(
        &'s self,
        db: &'s dyn HirDatabase,
    ) -> impl Iterator<Item = mun_hir::FileId> + 's {
        self.ordered_modules
            .iter()
            .filter_map(move |module| module.file_id(db))
    }

    /// Returns the filename for this module group
    pub fn relative_file_path(&self) -> mun_paths::RelativePathBuf {
        mun_paths::RelativePathBuf::from(self.name.replace("::", "$"))
    }
}

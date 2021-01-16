//! A `ModuleGroup` describes a grouping of modules that together form an assembly.

use hir::{HasVisibility, HirDatabase};
use rustc_hash::{FxHashMap, FxHashSet};
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;

/// A `ModuleGroup` describes a grouping of modules
#[derive(Clone, Eq, Debug)]
pub struct ModuleGroup {
    ordered_modules: Vec<hir::Module>,
    modules: FxHashSet<hir::Module>,
    includes_entire_subtree: FxHashMap<hir::Module, bool>,
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
    /// Constructs
    pub fn new(
        db: &dyn HirDatabase,
        name: String,
        modules: impl IntoIterator<Item = hir::Module>,
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

        let mut ordered_modules: Vec<hir::Module> = modules.iter().copied().collect();
        ordered_modules.sort();

        Self {
            ordered_modules,
            name,
            includes_entire_subtree,
            modules,
        }
    }

    /// Constructs a new module group from a single module
    pub fn from_single_module(db: &dyn HirDatabase, module: hir::Module) -> Self {
        Self::new(db, module.full_name(db), vec![module])
    }

    /// Returns true if the specified `hir::Module` is part of this group.
    pub fn contains(&self, module: hir::Module) -> bool {
        self.modules.contains(&module)
    }

    /// Returns an iterator over all modules in the group
    pub fn iter(&self) -> impl Iterator<Item = hir::Module> + '_ {
        self.ordered_modules.iter().copied()
    }

    /// Returns true if the specified function should be exported from the module group. This
    /// indicates that when queried the resulting assembly will expose this function.
    pub fn should_export_fn(&self, db: &dyn HirDatabase, function: hir::Function) -> bool {
        // If the function is not defined in the module group we should definitely not export it.
        if !self.modules.contains(&function.module(db)) {
            return false;
        }

        let vis = function.visibility(db);
        match vis {
            // If the function is publicly accessible it must always be exported
            hir::Visibility::Public => true,

            // The function is visible from the specified module and all child modules.
            hir::Visibility::Module(visible_mod) => {
                // If the modules is contained within `includes_entire_subtree` it is includes in
                // the module group.
                self.includes_entire_subtree
                    .get(&visible_mod.into())
                    // If all its children are also part of the module group we can keep the
                    // function internal, so there is no need to export it.
                    .map(|&includes_subtree| !includes_subtree)
                    // Otherwise, if it the module is not part of the group we have to export it.
                    .unwrap_or(true)
            }
        }
    }

    /// Returns true if the specified function should be included in the dispatch table of this
    /// module group if it is used from within this module group.
    pub fn should_runtime_link_fn(&self, db: &dyn HirDatabase, function: hir::Function) -> bool {
        function.is_extern(db) || !self.modules.contains(&function.module(db))
    }

    /// Returns the `hir::FileId` that are included in this module group
    pub fn files<'s>(&'s self, db: &'s dyn HirDatabase) -> impl Iterator<Item = hir::FileId> + 's {
        self.ordered_modules
            .iter()
            .filter_map(move |module| module.file_id(db))
    }
}

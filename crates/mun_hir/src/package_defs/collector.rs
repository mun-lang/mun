use super::PackageDefs;
use crate::{
    arena::map::ArenaMap,
    ids::{FunctionLoc, Intern, ItemDefinitionId, ModuleId, StructLoc, TypeAliasLoc},
    item_scope::ItemScope,
    item_tree::{
        Function, ItemTree, ItemTreeId, LocalItemTreeId, ModItem, Struct, StructDefKind, TypeAlias,
    },
    module_tree::{LocalModuleId, ModuleTree},
    visibility::RawVisibility,
    DefDatabase, FileId, Name, PackageId, PerNs, Visibility,
};
use std::sync::Arc;

pub(super) fn collect(db: &dyn DefDatabase, package_id: PackageId) -> PackageDefs {
    let mut collector = DefCollector {
        db,
        package_id,
        modules: Default::default(),
        module_tree: db.module_tree(package_id),
    };
    collector.collect();
    collector.finish()
}

/// A helper struct to collect all definitions for all modules in a package.
struct DefCollector<'db> {
    db: &'db dyn DefDatabase,
    package_id: PackageId,
    modules: ArenaMap<LocalModuleId, ItemScope>,
    module_tree: Arc<ModuleTree>,
}

impl<'db> DefCollector<'db> {
    /// Collects all information and stores it in the instance
    fn collect(&mut self) {
        // Collect all definitions in each module
        let module_tree = self.module_tree.clone();

        fn collect_modules_recursive(
            collector: &mut DefCollector,
            module_id: LocalModuleId,
            parent: Option<(Name, LocalModuleId)>,
        ) {
            // Insert an empty item scope for this module, this will be filled in.
            collector.modules.insert(module_id, ItemScope::default());

            // If there is a file associated with the module, collect all definitions from it
            let module_data = &collector.module_tree[module_id];
            if let Some(file_id) = module_data.file {
                let item_tree = collector.db.item_tree(file_id);
                let mut mod_collector = ModCollectorContext {
                    def_collector: collector,
                    module_id,
                    file_id,
                    item_tree: &item_tree,
                };

                mod_collector.collect(item_tree.top_level_items());
            }

            // Insert this module into the scope of the parent
            if let Some((name, parent)) = parent {
                collector.modules[parent].add_resolution(
                    name,
                    PerNs::from_definition(
                        ModuleId {
                            package: collector.package_id,
                            local_id: module_id,
                        }
                        .into(),
                        Visibility::Public,
                        false,
                    ),
                );
            }

            // Iterate over all children
            let child_module_ids = collector.module_tree[module_id]
                .children
                .iter()
                .map(|(name, local_id)| (name.clone(), *local_id))
                .collect::<Vec<_>>();
            for (name, child_module_id) in child_module_ids {
                collect_modules_recursive(collector, child_module_id, Some((name, module_id)));
            }
        };

        collect_modules_recursive(self, module_tree.root, None);
    }

    /// Create the `PackageDefs` struct that holds all the items
    fn finish(self) -> PackageDefs {
        PackageDefs {
            modules: self.modules,
            module_tree: self.module_tree,
        }
    }
}

/// Collects all items from a module
struct ModCollectorContext<'a, 'db> {
    def_collector: &'a mut DefCollector<'db>,
    module_id: LocalModuleId,
    file_id: FileId,
    item_tree: &'a ItemTree,
}

impl<'a> ModCollectorContext<'a, '_> {
    fn collect(&mut self, items: &[ModItem]) {
        for &item in items {
            let definition = match item {
                ModItem::Function(id) => self.collect_function(id),
                ModItem::Struct(id) => self.collect_struct(id),
                ModItem::TypeAlias(id) => self.collect_type_alias(id),
            };

            if let Some(DefData {
                id,
                name,
                visibility,
                has_constructor,
            }) = definition
            {
                self.def_collector.modules[self.module_id].add_definition(id);
                let visibility = self.def_collector.module_tree.resolve_visibility(
                    self.def_collector.db,
                    self.module_id,
                    visibility,
                );
                self.def_collector.modules[self.module_id].add_resolution(
                    name.clone(),
                    PerNs::from_definition(id, visibility, has_constructor),
                )
            }
        }
    }

    /// Collects the definition data from a `Function`
    fn collect_function(&self, id: LocalItemTreeId<Function>) -> Option<DefData<'a>> {
        let func = &self.item_tree[id];
        Some(DefData {
            id: FunctionLoc {
                module: ModuleId {
                    package: self.def_collector.package_id,
                    local_id: self.module_id,
                },
                id: ItemTreeId::new(self.file_id, id),
            }
            .intern(self.def_collector.db)
            .into(),
            name: &func.name,
            visibility: &self.item_tree[func.visibility],
            has_constructor: false,
        })
    }

    /// Collects the definition data from a `Struct`
    fn collect_struct(&self, id: LocalItemTreeId<Struct>) -> Option<DefData<'a>> {
        let adt = &self.item_tree[id];
        Some(DefData {
            id: StructLoc {
                module: ModuleId {
                    package: self.def_collector.package_id,
                    local_id: self.module_id,
                },
                id: ItemTreeId::new(self.file_id, id),
            }
            .intern(self.def_collector.db)
            .into(),
            name: &adt.name,
            visibility: &self.item_tree[adt.visibility],
            has_constructor: adt.kind != StructDefKind::Record,
        })
    }

    /// Collects the definition data from a `TypeAlias`
    fn collect_type_alias(&self, id: LocalItemTreeId<TypeAlias>) -> Option<DefData<'a>> {
        let type_alias = &self.item_tree[id];
        Some(DefData {
            id: TypeAliasLoc {
                module: ModuleId {
                    package: self.def_collector.package_id,
                    local_id: self.module_id,
                },
                id: ItemTreeId::new(self.file_id, id),
            }
            .intern(self.def_collector.db)
            .into(),
            name: &type_alias.name,
            visibility: &self.item_tree[type_alias.visibility],
            has_constructor: false,
        })
    }
}

struct DefData<'a> {
    id: ItemDefinitionId,
    name: &'a Name,
    visibility: &'a RawVisibility,
    has_constructor: bool,
}

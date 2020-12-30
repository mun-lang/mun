use super::PackageDefs;
use crate::item_scope::PerNsGlobImports;
use crate::{
    ids::ItemDefinitionId,
    ids::{FunctionLoc, Intern, StructLoc, TypeAliasLoc},
    item_scope::ImportType,
    item_scope::ItemScope,
    item_tree::{
        self, Function, ItemTree, ItemTreeId, LocalItemTreeId, ModItem, Struct, StructDefKind,
        TypeAlias,
    },
    module_tree::LocalModuleId,
    name_resolution::ReachedFixedPoint,
    package_defs::diagnostics::DefDiagnostic,
    path::ImportAlias,
    visibility::RawVisibility,
    DefDatabase, FileId, InFile, ModuleId, Name, PackageId, Path, PerNs, Visibility,
};
use rustc_hash::FxHashMap;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum PartialResolvedImport {
    /// None of any namespaces is resolved
    Unresolved,
    /// One of namespaces is resolved
    Indeterminate(PerNs<(ItemDefinitionId, Visibility)>),
    /// All namespaces are resolved, OR it is came from other crate
    Resolved(PerNs<(ItemDefinitionId, Visibility)>),
}

/// The result of an import directive
#[derive(Clone, Debug, Eq, PartialEq)]
struct ImportResolution {
    /// The name to expose the resolution as
    name: Option<Name>,

    /// The resolution itself
    resolution: PerNs<(ItemDefinitionId, Visibility)>,
}

impl PartialResolvedImport {
    fn namespaces(&self) -> PerNs<(ItemDefinitionId, Visibility)> {
        match self {
            PartialResolvedImport::Unresolved => PerNs::none(),
            PartialResolvedImport::Indeterminate(ns) => *ns,
            PartialResolvedImport::Resolved(ns) => *ns,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Import {
    /// The path of the import (e.g. foo::Bar). Note that group imports have been desugared, each
    /// item in the import tree is a seperate import.
    pub path: Path,

    /// The alias for this import statement
    pub alias: Option<ImportAlias>,

    /// The visibility of the import in the file the import statement resides in
    pub visibility: RawVisibility,

    /// Whether or not this is a * import.
    pub is_glob: bool,

    /// The original location of the import
    source: ItemTreeId<item_tree::Import>,
}

impl Import {
    /// Constructs an `Import` from a `use` statement in an `ItemTree`.
    fn from_use(tree: &ItemTree, id: ItemTreeId<item_tree::Import>) -> Self {
        let it = &tree[id.value];
        let visibility = &tree[it.visibility];
        Self {
            path: it.path.clone(),
            alias: it.alias.clone(),
            visibility: visibility.clone(),
            is_glob: it.is_glob,
            source: id,
        }
    }
}

/// A struct that keeps track of the state of an import directive.
#[derive(Clone, Debug, Eq, PartialEq)]
struct ImportDirective {
    /// The module that defines the import statement
    module_id: LocalModuleId,

    /// Information about the import statement.
    import: Import,

    /// The current status of the import.
    status: PartialResolvedImport,
}

pub(super) fn collect(db: &dyn DefDatabase, package_id: PackageId) -> PackageDefs {
    let mut collector = DefCollector {
        db,
        package_id,
        package_defs: PackageDefs {
            modules: Default::default(),
            module_tree: db.module_tree(package_id),
            diagnostics: Default::default(),
        },
        unresolved_imports: Default::default(),
        resolved_imports: Default::default(),
        glob_imports: Default::default(),
        from_glob_import: Default::default(),
    };
    collector.collect();
    collector.finish()
}

/// A helper struct to collect all definitions for all modules in a package.
struct DefCollector<'db> {
    db: &'db dyn DefDatabase,
    package_id: PackageId,
    package_defs: PackageDefs,
    // modules: ArenaMap<LocalModuleId, ItemScope>,
    // module_tree: Arc<ModuleTree>,
    unresolved_imports: Vec<ImportDirective>,
    resolved_imports: Vec<ImportDirective>,

    /// A mapping from local module to wildcard imports to other modules
    glob_imports: FxHashMap<LocalModuleId, Vec<(LocalModuleId, Visibility)>>,
    from_glob_import: PerNsGlobImports,
}

impl<'db> DefCollector<'db> {
    /// Collects all information and stores it in the instance
    fn collect(&mut self) {
        // Collect all definitions in each module
        let module_tree = self.package_defs.module_tree.clone();

        // Start by collecting the definitions from all modules. This ensures that very every module
        // all local definitions are accessible. This is the starting point for the import
        // resolution.
        collect_modules_recursive(self, module_tree.root, None);

        // Now, as long as we have unresolved imports, try to resolve them, or part of them.
        while !self.unresolved_imports.is_empty() {
            // Keep track of whether we were able to resolve anything
            let mut resolved_something = false;

            // Get all the current unresolved import directives
            let imports = std::mem::replace(&mut self.unresolved_imports, Vec::new());

            // For each import, try to resolve it with the current state.
            for mut directive in imports {
                // Resolve the import
                directive.status = self.resolve_import(directive.module_id, &directive.import);

                // Check the status of the import, if the import is still considered unresolved, try
                // again in the next round.
                match directive.status {
                    PartialResolvedImport::Indeterminate(_) => {
                        self.record_resolved_import(&directive);
                        // FIXME: For avoid performance regression,
                        // we consider an imported resolved if it is indeterminate (i.e not all namespace resolved)
                        self.resolved_imports.push(directive);
                        resolved_something = true;
                    }
                    PartialResolvedImport::Resolved(_) => {
                        self.record_resolved_import(&directive);
                        self.resolved_imports.push(directive);
                        resolved_something = true;
                    }
                    PartialResolvedImport::Unresolved => {
                        self.unresolved_imports.push(directive);
                    }
                }
            }

            if !resolved_something {
                break;
            }
        }

        fn collect_modules_recursive(
            collector: &mut DefCollector,
            module_id: LocalModuleId,
            parent: Option<(Name, LocalModuleId)>,
        ) {
            // Insert an empty item scope for this module, this will be filled in.
            collector
                .package_defs
                .modules
                .insert(module_id, ItemScope::default());

            // If there is a file associated with the module, collect all definitions from it
            let module_data = &collector.package_defs.module_tree[module_id];
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
                collector.package_defs.modules[parent].add_resolution(
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
            let child_module_ids = collector.package_defs.module_tree[module_id]
                .children
                .iter()
                .map(|(name, local_id)| (name.clone(), *local_id))
                .collect::<Vec<_>>();
            for (name, child_module_id) in child_module_ids {
                collect_modules_recursive(collector, child_module_id, Some((name, module_id)));
            }
        };
    }

    /// Given an import, try to resolve it.
    fn resolve_import(&self, module_id: LocalModuleId, import: &Import) -> PartialResolvedImport {
        let res = self
            .package_defs
            .resolve_path_with_fixedpoint(self.db, module_id, &import.path);

        let def = res.resolved_def;
        if res.reached_fixedpoint == ReachedFixedPoint::No || def.is_none() {
            return PartialResolvedImport::Unresolved;
        }

        if let Some(package) = res.package {
            if package != self.package_defs.module_tree.package {
                return PartialResolvedImport::Resolved(def);
            }
        }

        // Check whether all namespace is resolved
        if def.take_types().is_some() && def.take_values().is_some() {
            PartialResolvedImport::Resolved(def)
        } else {
            PartialResolvedImport::Indeterminate(def)
        }
    }

    /// Records ands propagates the resolution of an import directive.
    fn record_resolved_import(&mut self, directive: &ImportDirective) {
        let import_module_id = directive.module_id;
        let import = &directive.import;

        // Get the resolved definition of the use statement
        let resolution = directive.status.namespaces();

        // Get the visibility of the import statement
        let import_visibility = self.package_defs.module_tree.resolve_visibility(
            self.db,
            import_module_id,
            &directive.import.visibility,
        );

        if import.is_glob {
            match resolution.take_types() {
                Some((ItemDefinitionId::ModuleId(m), _)) => {
                    let scope = &self.package_defs[m.local_id];

                    // Get all the items that are visible from this module
                    let resolutions = scope
                        .entries()
                        .map(|(n, res)| ImportResolution {
                            name: Some(n.clone()),
                            resolution: res.and_then(|(item, vis)| {
                                if vis.is_visible_from_module_tree(
                                    &self.package_defs.module_tree,
                                    import_module_id,
                                ) {
                                    Some((item, vis))
                                } else {
                                    None
                                }
                            }),
                        })
                        .filter(|res| !res.resolution.is_none())
                        .collect::<Vec<_>>();

                    self.update(
                        import_module_id,
                        import_visibility,
                        ImportType::Glob,
                        &resolutions,
                    );

                    // Record the wildcard import in case new items are added to the module we are importing
                    let glob = self.glob_imports.entry(m.local_id).or_default();
                    if !glob.iter().any(|(m, _)| *m == import_module_id) {
                        glob.push((import_module_id, import_visibility));
                    }
                }
                Some((_, _)) => {
                    // Happens when wildcard importing something other than a module. I guess its ok to do nothing here?
                }
                None => {
                    // Happens if a wildcard import refers to something other than a type?
                }
            }
        } else {
            match import.path.segments.last() {
                Some(last_segment) => {
                    let name = match &import.alias {
                        Some(ImportAlias::Alias(name)) => Some(name.clone()),
                        Some(ImportAlias::Underscore) => None,
                        None => Some(last_segment.clone()),
                    };

                    self.update(
                        import_module_id,
                        import_visibility,
                        ImportType::Named,
                        &[ImportResolution { name, resolution }],
                    );
                }
                None => unreachable!(),
            }
        }
    }

    /// Updates the current state with the resolutions of an import statement.
    fn update(
        &mut self,
        import_module_id: LocalModuleId,
        import_visibility: Visibility,
        import_type: ImportType,
        resolutions: &[ImportResolution],
    ) {
        self.update_recursive(
            import_module_id,
            import_visibility,
            import_type,
            resolutions,
            0,
        );
    }

    /// Updates the current state with the resolutions of an import statement. Also recursively
    /// updates any wildcard imports.
    fn update_recursive(
        &mut self,
        import_module_id: LocalModuleId,
        import_visibility: Visibility,
        import_type: ImportType,
        resolutions: &[ImportResolution],
        depth: usize,
    ) {
        if depth > 100 {
            // prevent stack overflows (but this shouldn't be possible)
            panic!("infinite recursion in glob imports!");
        }

        let scope = &mut self.package_defs.modules[import_module_id];

        let mut changed = false;
        for ImportResolution { name, resolution } in resolutions {
            // TODO: Add an error if the visibility of the item does not allow exposing with the
            // import visibility. e.g.:
            // ```mun
            // //- foo.mun
            // pub(package) struct Foo;
            //
            // //- main.mun
            // pub foo::Foo; // This is not allowed because Foo is only public for the package.
            // ```

            match name {
                Some(name) => {
                    changed |= scope.add_resolution_from_import(
                        &mut self.from_glob_import,
                        (import_module_id, name.clone()),
                        resolution.map(|(item, _)| (item, import_visibility)),
                        import_type,
                    );
                }
                None => {
                    // This is not yet implemented (bringing in types into scope without a name).
                    // This might be useful for traits. e.g.:
                    // ```mun
                    // use foo::SomeTrait as _; // Should be able to call methods added by SomeTrait.
                    // ```
                    continue;
                }
            }
        }

        // If nothing changed, there is also no point in updating the wildcard imports
        if !changed {
            return;
        }

        let glob_imports = self
            .glob_imports
            .get(&import_module_id)
            .into_iter()
            .flat_map(|v| v.iter())
            .filter(|(glob_importing_module, _)| {
                import_visibility.is_visible_from_module_tree(
                    &self.package_defs.module_tree,
                    *glob_importing_module,
                )
            })
            .cloned()
            .collect::<Vec<_>>();

        for (glob_importing_module, glob_import_vis) in glob_imports {
            self.update_recursive(
                glob_importing_module,
                glob_import_vis,
                ImportType::Glob,
                resolutions,
                depth + 1,
            )
        }
    }

    /// Create the `PackageDefs` struct that holds all the items
    fn finish(self) -> PackageDefs {
        let mut package_defs = self.package_defs;

        // Create diagnostics for all unresolved imports
        for directive in self.unresolved_imports.iter() {
            let import = &directive.import;
            let item_tree = self.db.item_tree(import.source.file_id);
            let import_data = &item_tree[import.source.value];

            package_defs
                .diagnostics
                .push(DefDiagnostic::unresolved_import(
                    directive.module_id,
                    InFile::new(import.source.file_id, import_data.ast_id),
                    import_data.index,
                ))
        }

        package_defs
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
                ModItem::Import(id) => self.collect_import(id),
            };

            if let Some(DefData {
                id,
                name,
                visibility,
                has_constructor,
            }) = definition
            {
                self.def_collector.package_defs.modules[self.module_id].add_definition(id);
                let visibility = self
                    .def_collector
                    .package_defs
                    .module_tree
                    .resolve_visibility(self.def_collector.db, self.module_id, visibility);
                self.def_collector.package_defs.modules[self.module_id].add_resolution(
                    name.clone(),
                    PerNs::from_definition(id, visibility, has_constructor),
                );
            }
        }
    }

    /// Collects the definition data from an import statement.
    fn collect_import(&mut self, id: LocalItemTreeId<item_tree::Import>) -> Option<DefData<'a>> {
        self.def_collector.unresolved_imports.push(ImportDirective {
            module_id: self.module_id,
            import: Import::from_use(&self.item_tree, InFile::new(self.file_id, id)),
            status: PartialResolvedImport::Unresolved,
        });
        None
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

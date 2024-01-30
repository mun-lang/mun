use rustc_hash::FxHashMap;

use super::PackageDefs;
use crate::{
    arena::map::ArenaMap,
    ids::{
        FunctionLoc, ImplLoc, Intern, ItemContainerId, ItemDefinitionId, StructLoc, TypeAliasLoc,
    },
    item_scope::{ImportType, ItemScope, PerNsGlobImports},
    item_tree::{
        self, Fields, Function, Impl, ItemTree, ItemTreeId, LocalItemTreeId, ModItem, Struct,
        TypeAlias,
    },
    module_tree::LocalModuleId,
    name_resolution::ReachedFixedPoint,
    package_defs::diagnostics::DefDiagnostic,
    path::ImportAlias,
    visibility::RawVisibility,
    DefDatabase, FileId, InFile, ModuleId, Name, PackageId, Path, PerNs, Visibility,
};

/// Result of resolving an import statement
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum PartiallyResolvedImport {
    /// None of the namespaces are resolved
    Unresolved,
    /// One of namespaces is resolved.
    Indeterminate(PerNs<(ItemDefinitionId, Visibility)>),
    /// All namespaces are resolved, OR it came from another crate
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

impl PartiallyResolvedImport {
    fn namespaces(&self) -> PerNs<(ItemDefinitionId, Visibility)> {
        match self {
            PartiallyResolvedImport::Unresolved => PerNs::none(),
            PartiallyResolvedImport::Indeterminate(ns) | PartiallyResolvedImport::Resolved(ns) => {
                *ns
            }
        }
    }
}

/// Definition of a single import statement
#[derive(Clone, Debug, Eq, PartialEq)]
struct Import {
    /// The path of the import (e.g. foo::Bar). Note that group imports have
    /// been desugared, each item in the import tree is a seperate import.
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
    status: PartiallyResolvedImport,
}

pub(super) fn collect(db: &dyn DefDatabase, package_id: PackageId) -> PackageDefs {
    let mut collector = DefCollector {
        db,
        package_id,
        package_defs: PackageDefs {
            id: package_id,
            modules: ArenaMap::default(),
            module_tree: db.module_tree(package_id),
            diagnostics: Vec::default(),
        },
        unresolved_imports: Vec::default(),
        resolved_imports: Vec::default(),
        glob_imports: FxHashMap::default(),
        from_glob_import: PerNsGlobImports::default(),
    };
    collector.collect();
    collector.finish()
}

/// A helper struct to collect all definitions for all modules in a package.
struct DefCollector<'db> {
    db: &'db dyn DefDatabase,
    package_id: PackageId,
    package_defs: PackageDefs,
    unresolved_imports: Vec<ImportDirective>,
    resolved_imports: Vec<ImportDirective>,

    /// A mapping from local module to wildcard imports of other modules
    glob_imports:
        FxHashMap<LocalModuleId, Vec<(LocalModuleId, Visibility, ItemTreeId<item_tree::Import>)>>,

    /// A list of all items that have been imported via a wildcard
    from_glob_import: PerNsGlobImports,
}

impl<'db> DefCollector<'db> {
    /// Collects all information and stores it in the instance
    fn collect(&mut self) {
        /// Recursively iterate over all modules in the `ModuleTree` and add
        /// them and their definitions to their corresponding
        /// `ItemScope`.
        fn collect_modules_recursive(
            collector: &mut DefCollector<'_>,
            module_id: LocalModuleId,
            parent: Option<(Name, LocalModuleId)>,
        ) {
            // Insert an empty item scope for this module, this will be filled in.
            collector
                .package_defs
                .modules
                .insert(module_id, ItemScope::default());

            // If there is a file associated with the module, collect all definitions from
            // it
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
        }

        // Collect all definitions in each module
        let module_tree = self.package_defs.module_tree.clone();

        // Start by collecting the definitions from all modules. This ensures that, for
        // every module, all local definitions are accessible. This is the
        // starting point for the import resolution.
        collect_modules_recursive(self, module_tree.root, None);

        // Now, as long as we have unresolved imports, try to resolve them, or part of
        // them.
        while !self.unresolved_imports.is_empty() {
            // Keep track of whether we were able to resolve anything
            let mut resolved_something = false;

            // Get all the current unresolved import directives
            let imports = std::mem::take(&mut self.unresolved_imports);

            // For each import, try to resolve it with the current state.
            for mut directive in imports {
                // Resolve the import
                directive.status = self.resolve_import(directive.module_id, &directive.import);

                // Check the status of the import, if the import is still considered unresolved,
                // try again in the next round.
                #[allow(clippy::match_same_arms)]
                match directive.status {
                    PartiallyResolvedImport::Indeterminate(_) => {
                        self.record_resolved_import(&directive);
                        // FIXME: To avoid performance regression, we consider an import resolved
                        // if it is indeterminate (i.e not all namespace resolved). This might not
                        // completely resolve correctly in the future if we can have values and
                        // types with the same name.
                        self.resolved_imports.push(directive);
                        resolved_something = true;
                    }
                    PartiallyResolvedImport::Resolved(_) => {
                        self.record_resolved_import(&directive);
                        self.resolved_imports.push(directive);
                        resolved_something = true;
                    }
                    PartiallyResolvedImport::Unresolved => {
                        self.unresolved_imports.push(directive);
                    }
                }
            }

            // If nothing actually changed up to this point, stop resolving.
            if !resolved_something {
                break;
            }
        }
    }

    /// Given an import, try to resolve it.
    fn resolve_import(&self, module_id: LocalModuleId, import: &Import) -> PartiallyResolvedImport {
        let res = self
            .package_defs
            .resolve_path_with_fixedpoint(self.db, module_id, &import.path);

        let def = res.resolved_def;
        if res.reached_fixedpoint == ReachedFixedPoint::No || def.is_none() {
            return PartiallyResolvedImport::Unresolved;
        }

        if let Some(package) = res.package {
            if package != self.package_defs.module_tree.package {
                return PartiallyResolvedImport::Resolved(def);
            }
        }

        // Check whether all namespaces have been resolved
        if def.take_types().is_some() && def.take_values().is_some() {
            PartiallyResolvedImport::Resolved(def)
        } else {
            PartiallyResolvedImport::Indeterminate(def)
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
            #[allow(clippy::match_same_arms)]
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
                        import.source,
                        &resolutions,
                    );

                    // Record the wildcard import in case new items are added to the module we are
                    // importing
                    let glob = self.glob_imports.entry(m.local_id).or_default();
                    if !glob.iter().any(|(m, _, _)| *m == import_module_id) {
                        glob.push((import_module_id, import_visibility, import.source));
                    }
                }
                Some((_, _)) => {
                    // Happens when wildcard importing something other than a
                    // module. I guess it's ok to do nothing here?
                }
                None => {
                    // Happens if a wildcard import refers to something other
                    // than a type?
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
                        import.source,
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
        import_source: ItemTreeId<item_tree::Import>,
        resolutions: &[ImportResolution],
    ) {
        self.update_recursive(
            import_module_id,
            import_visibility,
            import_type,
            import_source,
            resolutions,
            0,
        );
    }

    /// Updates the current state with the resolutions of an import statement.
    /// Also recursively updates any wildcard imports.
    fn update_recursive(
        &mut self,
        import_module_id: LocalModuleId,
        import_visibility: Visibility,
        import_type: ImportType,
        import_source: ItemTreeId<item_tree::Import>,
        resolutions: &[ImportResolution],
        depth: usize,
    ) {
        // prevent stack overflows (but this shouldn't be possible)
        assert!(depth <= 100, "infinite recursion in glob imports!");

        let scope = &mut self.package_defs.modules[import_module_id];

        let mut changed = false;
        for ImportResolution { name, resolution } in resolutions {
            // TODO(#309): Add an error if the visibility of the item does not allow
            // exposing with the import visibility. e.g.:
            // ```mun
            // //- foo.mun
            // pub(package) struct Foo;
            //
            // //- main.mun
            // pub foo::Foo; // This is not allowed because Foo is only public within the package.
            // ```

            match name {
                Some(name) => {
                    let add_result = scope.add_resolution_from_import(
                        &mut self.from_glob_import,
                        (import_module_id, name.clone()),
                        resolution.map(|(item, _)| (item, import_visibility)),
                        import_type,
                    );

                    if add_result.changed {
                        changed = true;
                    }
                    if add_result.duplicate {
                        let item_tree = self.db.item_tree(import_source.file_id);
                        let import_data = &item_tree[import_source.value];
                        self.package_defs
                            .diagnostics
                            .push(DefDiagnostic::duplicate_import(
                                import_module_id,
                                InFile::new(import_source.file_id, import_data.ast_id),
                                import_data.index,
                            ));
                    }
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
            .filter(|(glob_importing_module, _, _)| {
                import_visibility.is_visible_from_module_tree(
                    &self.package_defs.module_tree,
                    *glob_importing_module,
                )
            })
            .cloned()
            .collect::<Vec<_>>();

        for (glob_importing_module, glob_import_vis, glob_import_source) in glob_imports {
            self.update_recursive(
                glob_importing_module,
                glob_import_vis,
                ImportType::Glob,
                glob_import_source,
                resolutions,
                depth + 1,
            );
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
                ));
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
            let DefData {
                id,
                name,
                visibility,
                has_constructor,
            } = match item {
                ModItem::Function(id) => self.collect_function(id),
                ModItem::Struct(id) => self.collect_struct(id),
                ModItem::TypeAlias(id) => self.collect_type_alias(id),
                ModItem::Import(id) => {
                    self.collect_import(id);
                    continue;
                }
                ModItem::Impl(id) => {
                    self.collect_impl(id);
                    continue;
                }
            };

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

    /// Collects the definition data from an `Impl`.
    fn collect_impl(&mut self, id: LocalItemTreeId<Impl>) {
        self.def_collector.package_defs.modules[self.module_id].define_impl(
            ImplLoc {
                module: ModuleId {
                    package: self.def_collector.package_id,
                    local_id: self.module_id,
                },
                id: ItemTreeId::new(self.file_id, id),
            }
            .intern(self.def_collector.db),
        );
    }

    /// Collects the definition data from an import statement.
    fn collect_import(&mut self, id: LocalItemTreeId<item_tree::Import>) {
        self.def_collector.unresolved_imports.push(ImportDirective {
            module_id: self.module_id,
            import: Import::from_use(self.item_tree, InFile::new(self.file_id, id)),
            status: PartiallyResolvedImport::Unresolved,
        });
    }

    /// Collects the definition data from a `Function`
    #[warn(clippy::unnecessary_wraps)]
    fn collect_function(&self, id: LocalItemTreeId<Function>) -> DefData<'a> {
        let func = &self.item_tree[id];
        DefData {
            id: FunctionLoc {
                container: ItemContainerId::ModuleId(ModuleId {
                    package: self.def_collector.package_id,
                    local_id: self.module_id,
                }),
                id: ItemTreeId::new(self.file_id, id),
            }
            .intern(self.def_collector.db)
            .into(),
            name: &func.name,
            visibility: &self.item_tree[func.visibility],
            has_constructor: false,
        }
    }

    /// Collects the definition data from a `Struct`
    fn collect_struct(&self, id: LocalItemTreeId<Struct>) -> DefData<'a> {
        let adt = &self.item_tree[id];
        DefData {
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
            has_constructor: !matches!(adt.fields, Fields::Record(_)),
        }
    }

    /// Collects the definition data from a `TypeAlias`
    fn collect_type_alias(&self, id: LocalItemTreeId<TypeAlias>) -> DefData<'a> {
        let type_alias = &self.item_tree[id];
        DefData {
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
        }
    }
}

struct DefData<'a> {
    id: ItemDefinitionId,
    name: &'a Name,
    visibility: &'a RawVisibility,
    has_constructor: bool,
}

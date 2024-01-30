use std::sync::Arc;

use crate::{
    expr::{scope::LocalScopeId, PatId},
    has_module::HasModule,
    ids::{
        DefWithBodyId, FunctionId, ImplId, ItemContainerId, ItemDefinitionId, Lookup, ModuleId,
        StructId, TypeAliasId,
    },
    item_scope::BUILTIN_SCOPE,
    module_tree::LocalModuleId,
    package_defs::PackageDefs,
    primitive_type::PrimitiveType,
    visibility::RawVisibility,
    DefDatabase, ExprId, ExprScopes, Name, Path, PerNs, Visibility,
};

#[derive(Debug, Clone, Default)]
pub struct Resolver {
    scopes: Vec<Scope>,
}

#[derive(Debug, Clone)]
pub(crate) enum Scope {
    /// All the items and imported names of a module
    ModuleScope(ModuleItemMap),

    /// Local bindings
    ExprScope(ExprScope),
}

#[derive(Debug, Clone)]
pub(crate) struct ModuleItemMap {
    package_defs: Arc<PackageDefs>,
    module_id: LocalModuleId,
}

#[derive(Debug, Clone)]
pub(crate) struct ExprScope {
    owner: DefWithBodyId,
    expr_scopes: Arc<ExprScopes>,
    scope_id: LocalScopeId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResolveValueResult {
    ValueNs(ValueNs, Visibility),
    Partial(TypeNs, usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValueNs {
    LocalBinding(PatId),
    FunctionId(FunctionId),
    StructId(StructId),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeNs {
    StructId(StructId),
    TypeAliasId(TypeAliasId),
    PrimitiveType(PrimitiveType),
}

/// An item definition visible from a certain scope.
pub enum ScopeDef {
    PerNs(PerNs<(ItemDefinitionId, Visibility)>),
    Local(PatId),
}

impl Resolver {
    /// Adds another scope to the resolver from which it can resolve names
    pub(crate) fn push_scope(mut self, scope: Scope) -> Resolver {
        self.scopes.push(scope);
        self
    }

    /// Adds a module scope to the resolver from which it can resolve names
    pub(crate) fn push_module_scope(
        self,
        package_defs: Arc<PackageDefs>,
        module_id: LocalModuleId,
    ) -> Resolver {
        self.push_scope(Scope::ModuleScope(ModuleItemMap {
            package_defs,
            module_id,
        }))
    }

    /// Adds an expression scope from which it can resolve names
    pub(crate) fn push_expr_scope(
        self,
        owner: DefWithBodyId,
        expr_scopes: Arc<ExprScopes>,
        scope_id: LocalScopeId,
    ) -> Resolver {
        self.push_scope(Scope::ExprScope(ExprScope {
            owner,
            expr_scopes,
            scope_id,
        }))
    }
}

impl Resolver {
    // TODO: This function is useful when we need to resolve paths between modules
    // /// Resolves a path
    // fn resolve_module_path(
    //     &self,
    //     db: &dyn DefDatabase,
    //     path: &Path,
    // ) -> PerNs<(ItemDefinitionId, Visibility)> {
    //     let (defs, module) = match self.module_scope() {
    //         None => return PerNs::none(),
    //         Some(it) => it,
    //     };
    //
    //     let (module_res, segment_index) = defs.resolve_path_in_module(db, module,
    // &path);
    //
    //     // If the `segment_index` contains a value it means the path didn't
    // resolve completely yet     if segment_index.is_some() {
    //         return PerNs::none();
    //     }
    //
    //     module_res
    // }

    /// Returns the `Module` scope of the resolver
    fn module_scope(&self) -> Option<(&PackageDefs, LocalModuleId)> {
        self.scopes.iter().rev().find_map(|scope| match scope {
            Scope::ModuleScope(m) => Some((&*m.package_defs, m.module_id)),
            Scope::ExprScope(_) => None,
        })
    }

    /// Resolves the visibility of the the `RawVisibility`
    pub fn resolve_visibility(
        &self,
        db: &dyn DefDatabase,
        visibility: &RawVisibility,
    ) -> Option<Visibility> {
        self.module_scope().map(|(package_defs, module)| {
            package_defs
                .module_tree
                .resolve_visibility(db, module, visibility)
        })
    }

    /// Resolves the specified `path` as a value. Returns a result that can also
    /// indicate that the path was only partially resolved.
    pub fn resolve_path_as_value(
        &self,
        db: &dyn DefDatabase,
        path: &Path,
    ) -> Option<ResolveValueResult> {
        fn to_value_ns(
            per_ns: PerNs<(ItemDefinitionId, Visibility)>,
        ) -> Option<(ValueNs, Visibility)> {
            let (res, vis) = match per_ns.take_values()? {
                (ItemDefinitionId::FunctionId(id), vis) => (ValueNs::FunctionId(id), vis),
                (ItemDefinitionId::StructId(id), vis) => (ValueNs::StructId(id), vis),
                (
                    ItemDefinitionId::ModuleId(_)
                    | ItemDefinitionId::TypeAliasId(_)
                    | ItemDefinitionId::PrimitiveType(_),
                    _,
                ) => return None,
            };
            Some((res, vis))
        }

        let segments_count = path.segments.len();
        let first_name = path.segments.first()?;
        for scope in self.scopes.iter().rev() {
            match scope {
                Scope::ExprScope(scope) if segments_count <= 1 => {
                    let entry = scope
                        .expr_scopes
                        .entries(scope.scope_id)
                        .iter()
                        .find(|entry| entry.name() == first_name);

                    if let Some(e) = entry {
                        return Some(ResolveValueResult::ValueNs(
                            ValueNs::LocalBinding(e.pat()),
                            Visibility::Public,
                        ));
                    }
                }
                Scope::ExprScope(_) => continue,

                Scope::ModuleScope(m) => {
                    let (module_def, idx) =
                        m.package_defs.resolve_path_in_module(db, m.module_id, path);
                    return match idx {
                        None => {
                            let (value, vis) = to_value_ns(module_def)?;
                            Some(ResolveValueResult::ValueNs(value, vis))
                        }
                        Some(idx) => {
                            let ty = match module_def.take_types()? {
                                (ItemDefinitionId::StructId(id), _) => TypeNs::StructId(id),
                                (ItemDefinitionId::TypeAliasId(id), _) => TypeNs::TypeAliasId(id),
                                (ItemDefinitionId::PrimitiveType(id), _) => {
                                    TypeNs::PrimitiveType(id)
                                }
                                (
                                    ItemDefinitionId::ModuleId(_) | ItemDefinitionId::FunctionId(_),
                                    _,
                                ) => return None,
                            };
                            Some(ResolveValueResult::Partial(ty, idx))
                        }
                    };
                }
            };
        }

        None
    }

    /// Resolves the specified `path` as a value. Returns either `None` or the
    /// resolved path value.
    pub fn resolve_path_as_value_fully(
        &self,
        db: &dyn DefDatabase,
        path: &Path,
    ) -> Option<(ValueNs, Visibility)> {
        match self.resolve_path_as_value(db, path)? {
            ResolveValueResult::ValueNs(val, vis) => Some((val, vis)),
            ResolveValueResult::Partial(..) => None,
        }
    }

    /// Resolves the specified `path` as a type. Returns a result that can also
    /// indicate that the path was only partially resolved.
    pub fn resolve_path_as_type(
        &self,
        db: &dyn DefDatabase,
        path: &Path,
    ) -> Option<(TypeNs, Visibility, Option<usize>)> {
        fn to_type_ns(
            per_ns: PerNs<(ItemDefinitionId, Visibility)>,
        ) -> Option<(TypeNs, Visibility)> {
            let (res, vis) = match per_ns.take_types()? {
                (ItemDefinitionId::StructId(id), vis) => (TypeNs::StructId(id), vis),
                (ItemDefinitionId::TypeAliasId(id), vis) => (TypeNs::TypeAliasId(id), vis),
                (ItemDefinitionId::PrimitiveType(id), vis) => (TypeNs::PrimitiveType(id), vis),
                (ItemDefinitionId::ModuleId(_) | ItemDefinitionId::FunctionId(_), _) => {
                    return None;
                }
            };
            Some((res, vis))
        }

        for scope in self.scopes.iter().rev() {
            match scope {
                Scope::ExprScope(_) => continue,
                Scope::ModuleScope(m) => {
                    let (module_def, idx) =
                        m.package_defs.resolve_path_in_module(db, m.module_id, path);
                    let (res, vis) = to_type_ns(module_def)?;
                    return Some((res, vis, idx));
                }
            }
        }

        None
    }

    /// Resolves the specified `path` as a type. Returns either `None` or the
    /// resolved path type.
    pub fn resolve_path_as_type_fully(
        &self,
        db: &dyn DefDatabase,
        path: &Path,
    ) -> Option<(TypeNs, Visibility)> {
        let (res, visibility, unresolved) = self.resolve_path_as_type(db, path)?;
        if unresolved.is_some() {
            return None;
        }
        Some((res, visibility))
    }

    /// Returns the module from which this instance resolves names
    pub fn module(&self) -> Option<ModuleId> {
        let (package_defs, local_id) = self.module_scope()?;
        Some(ModuleId {
            package: package_defs.module_tree.package,
            local_id,
        })
    }

    /// If the resolver holds a scope from a body, returns that body.
    pub fn body_owner(&self) -> Option<DefWithBodyId> {
        self.scopes.iter().rev().find_map(|scope| match scope {
            Scope::ExprScope(it) => Some(it.owner),
            Scope::ModuleScope(_) => None,
        })
    }

    /// Calls the `visitor` for each entry in scope.
    pub fn visit_all_names(&self, db: &dyn DefDatabase, visitor: &mut dyn FnMut(Name, ScopeDef)) {
        for scope in self.scopes.iter().rev() {
            scope.visit_names(db, visitor);
        }
    }
}

impl Scope {
    /// Calls the `visitor` for each entry in scope.
    fn visit_names(&self, _db: &dyn DefDatabase, visitor: &mut dyn FnMut(Name, ScopeDef)) {
        match self {
            Scope::ModuleScope(m) => {
                m.package_defs[m.module_id]
                    .entries()
                    .for_each(|(name, def)| visitor(name.clone(), ScopeDef::PerNs(def)));
                BUILTIN_SCOPE.iter().for_each(|(name, &def)| {
                    visitor(name.clone(), ScopeDef::PerNs(def));
                });
            }
            Scope::ExprScope(scope) => scope
                .expr_scopes
                .entries(scope.scope_id)
                .iter()
                .for_each(|entry| visitor(entry.name().clone(), ScopeDef::Local(entry.pat()))),
        }
    }
}

/// Returns a resolver applicable to the specified expression
pub fn resolver_for_expr(db: &dyn DefDatabase, owner: DefWithBodyId, expr_id: ExprId) -> Resolver {
    let scopes = db.expr_scopes(owner);
    resolver_for_scope(db, owner, scopes.scope_for(expr_id))
}

pub fn resolver_for_scope(
    db: &dyn DefDatabase,
    owner: DefWithBodyId,
    scope_id: Option<LocalScopeId>,
) -> Resolver {
    let mut r = owner.resolver(db);
    let scopes = db.expr_scopes(owner);
    let scope_chain = scopes.scope_chain(scope_id).collect::<Vec<_>>();
    r.scopes.reserve(scope_chain.len());

    for scope in scope_chain.into_iter().rev() {
        r = r.push_expr_scope(owner, Arc::clone(&scopes), scope);
    }
    r
}

pub trait HasResolver: Copy {
    /// Builds a resolver for type or value references inside this instance.
    fn resolver(self, db: &dyn DefDatabase) -> Resolver;
}

impl HasResolver for ModuleId {
    fn resolver(self, db: &dyn DefDatabase) -> Resolver {
        let defs = db.package_defs(self.package);
        Resolver::default().push_module_scope(defs, self.local_id)
    }
}

impl HasResolver for FunctionId {
    fn resolver(self, db: &dyn DefDatabase) -> Resolver {
        self.lookup(db).container.resolver(db)
    }
}

impl HasResolver for StructId {
    fn resolver(self, db: &dyn DefDatabase) -> Resolver {
        self.module(db).resolver(db)
    }
}

impl HasResolver for TypeAliasId {
    fn resolver(self, db: &dyn DefDatabase) -> Resolver {
        self.module(db).resolver(db)
    }
}

impl HasResolver for DefWithBodyId {
    fn resolver(self, db: &dyn DefDatabase) -> Resolver {
        match self {
            DefWithBodyId::FunctionId(f) => f.resolver(db),
        }
    }
}

impl HasResolver for ItemContainerId {
    fn resolver(self, db: &dyn DefDatabase) -> Resolver {
        match self {
            ItemContainerId::ModuleId(it) => it.resolver(db),
            ItemContainerId::ImplId(it) => it.resolver(db),
        }
    }
}

impl HasResolver for ImplId {
    fn resolver(self, db: &dyn DefDatabase) -> Resolver {
        self.module(db).resolver(db)
    }
}

use std::{
    collections::{hash_map::Entry, HashMap},
    ops::ControlFlow,
    sync::Arc,
};

use mun_hir_input::{ModuleId, PackageId, PackageModuleId};
use mun_syntax::{AstNode, AstPtr, SyntaxNodePtr};
use rustc_hash::FxHashMap;

use crate::{
    db::HirDatabase,
    diagnostics::{DuplicateDefinition, ImplForForeignType, InvalidSelfTyImpl},
    has_module::HasModule,
    ids::{AssocItemId, FunctionId, ImplId, Lookup, StructId},
    package_defs::PackageDefs,
    ty::lower::LowerDiagnostic,
    DefDatabase, DiagnosticSink, HasSource, InFile, Name, Ty, TyKind,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InherentImplsDiagnostics {
    /// An error occurred when resolving a type in an impl.
    LowerDiagnostic(ImplId, LowerDiagnostic),

    /// The type in the impl is not a valid type to implement.
    InvalidSelfTy(ImplId),

    /// The type in the impl is not defined in the same package as the impl.
    ImplForForeignType(ImplId),

    /// Duplicate definitions of an associated item
    DuplicateDefinitions(AssocItemId, AssocItemId),
}

/// Holds inherit impls defined in some package.
///
/// Inherent impls are impls that are defined for a type in the same package as
/// the type itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InherentImpls {
    map: FxHashMap<StructId, Vec<ImplId>>,
    diagnostics: Vec<InherentImplsDiagnostics>,
}

impl InherentImpls {
    /// A query function to extract all the inherent impls defined in a package.
    pub(crate) fn inherent_impls_in_package_query(
        db: &dyn HirDatabase,
        package: PackageId,
    ) -> Arc<Self> {
        let mut impls = Self {
            map: FxHashMap::default(),
            diagnostics: Vec::new(),
        };

        let package_defs = db.package_defs(package);
        impls.collect_from_package_defs(db, &package_defs);
        impls.shrink_to_fit();

        Arc::new(impls)
    }

    /// A method to ensure that this instance only uses the amount of memory it
    /// needs.
    ///
    /// This effectively removes all extra allocated capacity from the `map` and
    /// `invalid_impls` fields.
    fn shrink_to_fit(&mut self) {
        self.map.values_mut().for_each(Vec::shrink_to_fit);
        self.map.shrink_to_fit();
        self.diagnostics.shrink_to_fit();
    }

    /// Collects all the inherent impls defined in a package.
    fn collect_from_package_defs(&mut self, db: &dyn HirDatabase, package_defs: &PackageDefs) {
        for (_module_id, scope) in package_defs.modules.iter() {
            for impl_id in scope.impls() {
                let impl_data = db.impl_data(impl_id);

                // Resolve the self type of the impl
                let lowered = db.lower_impl(impl_id);
                self.diagnostics.extend(
                    lowered
                        .diagnostics
                        .iter()
                        .map(|d| InherentImplsDiagnostics::LowerDiagnostic(impl_id, d.clone())),
                );

                // Make sure the type is a struct
                let self_ty = lowered[impl_data.self_ty].clone();
                let s = match self_ty.interned() {
                    TyKind::Struct(s) => s,
                    TyKind::Unknown => continue,
                    _ => {
                        self.diagnostics
                            .push(InherentImplsDiagnostics::InvalidSelfTy(impl_id));
                        continue;
                    }
                };

                // Make sure the struct is defined in the same package
                if s.module(db).package().id != package_defs.id {
                    self.diagnostics
                        .push(InherentImplsDiagnostics::ImplForForeignType(impl_id));
                }

                // Add the impl to the map
                self.map.entry(s.id).or_default().push(impl_id);
            }
        }

        // Find duplicate associated items
        for (_, impls) in self.map.iter() {
            let mut name_to_item = HashMap::new();
            for impl_id in impls.iter() {
                let impl_data = db.impl_data(*impl_id);
                for item in impl_data.items.iter() {
                    let name = match item {
                        AssocItemId::FunctionId(it) => db.fn_data(*it).name().clone(),
                    };
                    match name_to_item.entry(name) {
                        Entry::Vacant(entry) => {
                            entry.insert(*item);
                        }
                        Entry::Occupied(entry) => {
                            self.diagnostics
                                .push(InherentImplsDiagnostics::DuplicateDefinitions(
                                    *entry.get(),
                                    *item,
                                ));
                        }
                    }
                }
            }
        }
    }

    /// Adds all the `InherentImplsDiagnostics`s of the result of a specific
    /// module to the `DiagnosticSink`.
    pub(crate) fn add_module_diagnostics(
        &self,
        db: &dyn HirDatabase,
        module_id: PackageModuleId,
        sink: &mut DiagnosticSink<'_>,
    ) {
        self.diagnostics
            .iter()
            .filter(|it| it.module_id(db).local_id == module_id)
            .for_each(|it| it.add_to(db, sink));
    }

    /// Adds all the `InherentImplsDiagnostics`s of the result to the
    /// `DiagnosticSink`.
    pub(crate) fn add_diagnostics(&self, db: &dyn HirDatabase, sink: &mut DiagnosticSink<'_>) {
        self.diagnostics.iter().for_each(|it| it.add_to(db, sink));
    }

    /// Returns all implementations defined in this instance.
    pub fn all_impls(&self) -> impl Iterator<Item = ImplId> + '_ {
        self.map.values().flatten().copied()
    }

    /// Returns all implementations defined for the specified type.
    pub fn for_self_ty(&self, self_ty: &Ty) -> &[ImplId] {
        match self_ty.interned() {
            TyKind::Struct(s) => self.map.get(&s.id).map_or(&[], AsRef::as_ref),
            _ => &[],
        }
    }
}

impl InherentImplsDiagnostics {
    fn add_to(&self, db: &dyn HirDatabase, sink: &mut DiagnosticSink<'_>) {
        match self {
            InherentImplsDiagnostics::LowerDiagnostic(impl_id, diag) => {
                let impl_data = db.impl_data(*impl_id);
                let file_id = impl_id.lookup(db).id.file_id;
                diag.add_to(db, file_id, &impl_data.type_ref_source_map, sink);
            }
            InherentImplsDiagnostics::InvalidSelfTy(impl_id) => sink.push(InvalidSelfTyImpl {
                impl_: impl_id.lookup(db).source(db).as_ref().map(AstPtr::new),
            }),
            InherentImplsDiagnostics::ImplForForeignType(impl_id) => {
                sink.push(ImplForForeignType {
                    impl_: impl_id.lookup(db).source(db).as_ref().map(AstPtr::new),
                });
            }
            InherentImplsDiagnostics::DuplicateDefinitions(first, second) => {
                sink.push(DuplicateDefinition {
                    definition: assoc_item_syntax_node_ptr(db, second),
                    first_definition: assoc_item_syntax_node_ptr(db, first),
                    name: assoc_item_name(db, first),
                });
            }
        }
    }

    fn module_id(&self, db: &dyn DefDatabase) -> ModuleId {
        match self {
            InherentImplsDiagnostics::LowerDiagnostic(impl_id, _)
            | InherentImplsDiagnostics::InvalidSelfTy(impl_id)
            | InherentImplsDiagnostics::ImplForForeignType(impl_id) => impl_id.module(db),
            InherentImplsDiagnostics::DuplicateDefinitions(_first, second) => second.module(db),
        }
    }
}

fn assoc_item_syntax_node_ptr(db: &dyn DefDatabase, id: &AssocItemId) -> InFile<SyntaxNodePtr> {
    match id {
        AssocItemId::FunctionId(it) => it
            .lookup(db)
            .source(db)
            .map(|node| SyntaxNodePtr::new(node.syntax())),
    }
}

fn assoc_item_name(db: &dyn DefDatabase, id: &AssocItemId) -> String {
    match id {
        AssocItemId::FunctionId(it) => db.fn_data(*it).name().to_string(),
    }
}

/// An object to iterate over methods associated with a type.
pub struct MethodResolutionCtx<'db> {
    pub db: &'db dyn HirDatabase,

    /// The type for which to resolve methods
    pub ty: Ty,

    /// Filter based on this name
    name: Option<Name>,

    /// Filter based on visibility from this module
    visible_from: Option<ModuleId>,

    /// Whether to look up methods or associated functions.
    association_mode: Option<AssociationMode>,
}

enum IsValidCandidate {
    Yes,
    No,
    NotVisible,
}

/// Whether to look up methods or associated functions.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AssociationMode {
    /// Method call e.g. a method that takes self as the first argument.
    WithSelf,

    /// Associated function e.g. a method that does not take self as the first
    /// argument.
    WithoutSelf,
}

impl<'db> MethodResolutionCtx<'db> {
    pub fn new(db: &'db dyn HirDatabase, ty: Ty) -> Self {
        Self {
            db,
            ty,
            name: None,
            visible_from: None,
            association_mode: None,
        }
    }

    /// Filter methods based on the specified lookup mode.
    pub fn with_association(self, association: AssociationMode) -> Self {
        Self {
            association_mode: Some(association),
            ..self
        }
    }

    /// Filter methods based on the specified lookup mode.
    pub fn with_association_opt(self, association: Option<AssociationMode>) -> Self {
        Self {
            association_mode: association,
            ..self
        }
    }

    /// Only include methods with the specified name.
    pub fn with_name(self, name: Name) -> Self {
        Self {
            name: Some(name),
            ..self
        }
    }

    /// Only include methods that are visible from the specified module.
    pub fn visible_from(self, module_id: ModuleId) -> Self {
        Self {
            visible_from: Some(module_id),
            ..self
        }
    }

    /// Collects all methods that match the specified criteria.
    ///
    /// If the callback method returns `Some(_)`, the iteration will stop and
    /// value will be returned.
    pub fn collect<T>(
        &self,
        mut callback: impl FnMut(AssocItemId, bool) -> Option<T>,
    ) -> Option<T> {
        match self.collect_inner(|item, visible| match callback(item, visible) {
            Some(r) => ControlFlow::Break(r),
            None => ControlFlow::Continue(()),
        }) {
            ControlFlow::Continue(()) => None,
            ControlFlow::Break(r) => Some(r),
        }
    }

    fn collect_inner<T>(
        &self,
        mut callback: impl FnMut(AssocItemId, bool) -> ControlFlow<T>,
    ) -> ControlFlow<T> {
        let Some(package_id) = self.defining_package() else {
            return ControlFlow::Continue(());
        };
        let inherent_impls = self.db.inherent_impls_in_package(package_id);
        let impls = inherent_impls.for_self_ty(&self.ty);
        for &self_impl in impls {
            let impl_data = self.db.impl_data(self_impl);
            for item in impl_data.items.iter().copied() {
                let visible = match self.is_valid_candidate(self_impl, item) {
                    IsValidCandidate::Yes => true,
                    IsValidCandidate::No => continue,
                    IsValidCandidate::NotVisible => false,
                };
                callback(item, visible)?;
            }
        }

        ControlFlow::Continue(())
    }

    /// Returns the package in which the type was defined.
    fn defining_package(&self) -> Option<PackageId> {
        match self.ty.interned() {
            TyKind::Struct(s) => {
                let module = s.module(self.db);
                Some(module.id.package)
            }
            _ => None,
        }
    }

    /// Returns whether the specified item is a valid candidate for method
    /// resolution based on the filters.
    fn is_valid_candidate(&self, impl_id: ImplId, item: AssocItemId) -> IsValidCandidate {
        match item {
            AssocItemId::FunctionId(f) => self.is_valid_function_candidate(impl_id, f),
        }
    }

    /// Returns true if the specified function is a valid candidate for method
    /// resolution based on the filters.
    fn is_valid_function_candidate(
        &self,
        _impl_id: ImplId,
        fun_id: FunctionId,
    ) -> IsValidCandidate {
        let data = self.db.fn_data(fun_id);

        // Check if the name matches
        if let Some(name) = &self.name {
            if data.name() != name {
                return IsValidCandidate::No;
            }
        }

        // Check the association mode
        if let Some(association_mode) = self.association_mode {
            if matches!(
                (association_mode, data.has_self_param()),
                (AssociationMode::WithSelf, false) | (AssociationMode::WithoutSelf, true)
            ) {
                return IsValidCandidate::No;
            }
        }

        // Check if the function is visible from the selected module
        if let Some(visible_from) = self.visible_from {
            if !self
                .db
                .function_visibility(fun_id)
                .is_visible_from(self.db, visible_from)
            {
                return IsValidCandidate::NotVisible;
            }
        }

        IsValidCandidate::Yes
    }
}

/// Find the method with the specified name on the specified type.
///
/// Returns `Ok` if the method was found, `Err(None)` if no method by that name
/// was found and `Err(Some(_))` if a method by that name was found but it is
/// not visible from the selected module.
pub(crate) fn lookup_method(
    db: &dyn HirDatabase,
    ty: &Ty,
    visible_from_module: ModuleId,
    name: &Name,
    association_mode: Option<AssociationMode>,
) -> Result<FunctionId, Option<FunctionId>> {
    let mut not_visible = None;
    MethodResolutionCtx::new(db, ty.clone())
        .with_association_opt(association_mode)
        .visible_from(visible_from_module)
        .with_name(name.clone())
        .collect(|item, visible| match item {
            AssocItemId::FunctionId(f) if visible => Some(f),
            AssocItemId::FunctionId(f) => {
                not_visible = Some(f);
                None
            }
        })
        .ok_or(not_visible)
}

#[cfg(test)]
mod tests {
    use mun_hir_input::{SourceDatabase, WithFixture};

    use crate::{
        code_model::AssocItem,
        display::HirDisplay,
        method_resolution::{lookup_method, MethodResolutionCtx},
        mock::MockDatabase,
        DiagnosticSink, HirDatabase, Module, ModuleDef, Name, Package, Ty,
    };

    #[test]
    fn test_query() {
        let db = MockDatabase::with_files(
            r#"
            //- /main.mun
            struct Foo;
            impl Foo {
                fn foo() {}
            }
            impl Foo {}
            "#,
        );

        let package_id = db.packages().iter().next().unwrap();
        let impls = db.inherent_impls_in_package(package_id);

        assert_eq!(impls.diagnostics, Vec::new());
        assert_eq!(impls.map.values().flatten().count(), 2);
    }

    fn impl_diagnostics(fixture: &str) -> String {
        let db = MockDatabase::with_files(fixture);

        let package_id = db.packages().iter().next().unwrap();
        let impls = db.inherent_impls_in_package(package_id);

        let mut diags = Vec::new();
        let mut diag_sink = DiagnosticSink::new(|diag| {
            diags.push(format!("{:?}: {}", diag.highlight_range(), diag.message()));
        });

        impls.add_diagnostics(&db, &mut diag_sink);

        drop(diag_sink);
        diags.join("\n")
    }

    #[test]
    fn test_doesnt_exist() {
        insta::assert_snapshot!(impl_diagnostics(r#"
            //- /main.mun
            impl DoesntExist {}
            "#),
            @"5..16: undefined type");
    }

    #[test]
    fn test_invalid_self_ty() {
        insta::assert_snapshot!(impl_diagnostics(r#"
            //- /main.mun
            struct Foo;
            impl i32 {}
            impl [Foo] {}
            "#),
            @r###"
        12..23: inherent `impl` blocks can only be added for structs
        24..37: inherent `impl` blocks can only be added for structs
        "###);
    }

    #[test]
    fn test_duplicate() {
        insta::assert_snapshot!(impl_diagnostics(r#"
            //- /main.mun
            struct Foo;
            impl Foo {
                fn bar();
                fn bar();
            }
            impl Foo {
                fn bar();
            }
            "#),
            @r###"
        36..50: the name `bar` is defined multiple times
        63..77: the name `bar` is defined multiple times
        "###);
    }

    struct Fixture {
        db: MockDatabase,
        root_module: Module,
        foo_ty: Ty,
    }

    impl Fixture {
        pub fn new() -> Self {
            let db = MockDatabase::with_files(
                r#"
            //- /mod.mun
            struct Foo;
            impl Foo {
                fn bar();
            }
            //- /foo.mun
            use super::Foo;

            impl Foo {
                fn baz(value: i32);
            }
            "#,
            );

            let package = Package::all(&db).into_iter().next().unwrap();
            let root_module = package.root_module(&db);

            let foo_ty = root_module
                .declarations(&db)
                .into_iter()
                .find_map(|decl| match decl {
                    ModuleDef::Struct(s) if s.name(&db).as_str() == Some("Foo") => Some(s.ty(&db)),
                    _ => None,
                })
                .unwrap();

            Self {
                db,
                root_module,
                foo_ty,
            }
        }
    }

    #[test]
    fn test_method_resolution_visibility() {
        let fixture = Fixture::new();

        insta::assert_snapshot!(
            display_method_resolution(
                MethodResolutionCtx::new(&fixture.db, fixture.foo_ty)
                    .visible_from(fixture.root_module.id)),
            @r###"
        + fn bar()
        - fn baz(value: i32)
        "###);
    }

    #[test]
    fn test_method_resolution_by_name() {
        let fixture = Fixture::new();

        insta::assert_snapshot!(
            display_method_resolution(
                MethodResolutionCtx::new(&fixture.db, fixture.foo_ty)
                    .with_name(Name::new("bar"))),
            @r###"
        + fn bar()
        "###);
    }

    #[test]
    fn test_lookup_method() {
        let fixture = Fixture::new();
        assert!(lookup_method(
            &fixture.db,
            &fixture.foo_ty,
            fixture.root_module.id,
            &Name::new("bar"),
            None,
        )
        .is_ok());
    }

    #[test]
    fn test_lookup_method_not_found() {
        let fixture = Fixture::new();
        assert!(lookup_method(
            &fixture.db,
            &fixture.foo_ty,
            fixture.root_module.id,
            &Name::new("not_found"),
            None,
        )
        .unwrap_err()
        .is_none());
    }

    #[test]
    fn test_lookup_method_not_visible() {
        let fixture = Fixture::new();
        assert!(lookup_method(
            &fixture.db,
            &fixture.foo_ty,
            fixture.root_module.id,
            &Name::new("baz"),
            None,
        )
        .unwrap_err()
        .is_some());
    }

    fn display_method_resolution(ctx: MethodResolutionCtx<'_>) -> String {
        let mut methods = Vec::new();
        ctx.collect(|item, visible| {
            methods.push(format!(
                "{}{}",
                if visible { "+ " } else { "- " },
                AssocItem::from(item).display(ctx.db)
            ));

            None::<()>
        });

        methods.join("\n")
    }
}

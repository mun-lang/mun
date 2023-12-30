use crate::diagnostics::{DuplicateDefinition, IncoherentImpl, InvalidSelfTyImpl};
use crate::ids::AssocItemId;
use crate::{
    db::HirDatabase,
    has_module::HasModule,
    ids::{ImplId, Lookup, StructId},
    module_tree::LocalModuleId,
    package_defs::PackageDefs,
    ty::lower::LowerDiagnostic,
    DefDatabase, DiagnosticSink, HasSource, InFile, ModuleId, PackageId, Ty, TyKind,
};
use mun_syntax::{AstNode, AstPtr, SyntaxNodePtr};
use rustc_hash::FxHashMap;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InherentImplsDiagnostics {
    /// An error occurred when resolving a type in an impl.
    LowerDiagnostic(ImplId, LowerDiagnostic),

    /// The type in the impl is not a valid type to implement.
    InvalidSelfTy(ImplId),

    /// The type in the impl is not defined in the same package as the impl.
    IncoherentType(ImplId),

    /// Duplicate definitions of an associated item
    DuplicateDefinitions(AssocItemId, AssocItemId),
}

/// Holds inherit impls defined in some package.
///
/// Inherent impls are impls that are defined for a type in the same package as the type itself.
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

    /// A method to ensure that this instance only uses the amount of memory it needs.
    ///
    /// This effectively removes all extra allocated capacity from the `map` and `invalid_impls` fields.
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
                        .push(InherentImplsDiagnostics::IncoherentType(impl_id));
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

    /// Adds all the `InherentImplsDiagnostics`s of the result of a specific module to the `DiagnosticSink`.
    pub(crate) fn add_module_diagnostics(
        &self,
        db: &dyn HirDatabase,
        module_id: LocalModuleId,
        sink: &mut DiagnosticSink<'_>,
    ) {
        self.diagnostics
            .iter()
            .filter(|it| it.module_id(db.upcast()).local_id == module_id)
            .for_each(|it| it.add_to(db, sink));
    }

    /// Adds all the `InherentImplsDiagnostics`s of the result to the `DiagnosticSink`.
    pub(crate) fn add_diagnostics(&self, db: &dyn HirDatabase, sink: &mut DiagnosticSink<'_>) {
        self.diagnostics.iter().for_each(|it| it.add_to(db, sink));
    }

    /// Returns all implementations defined in this instance.
    pub fn all_impls(&self) -> impl Iterator<Item = ImplId> + '_ {
        self.map.values().flatten().copied()
    }

    // Returns all implementations defined for the specified type.
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
                let file_id = impl_id.lookup(db.upcast()).id.file_id;
                diag.add_to(db, file_id, &impl_data.type_ref_source_map, sink);
            }
            InherentImplsDiagnostics::InvalidSelfTy(impl_id) => sink.push(InvalidSelfTyImpl {
                impl_: impl_id
                    .lookup(db.upcast())
                    .source(db.upcast())
                    .as_ref()
                    .map(AstPtr::new),
            }),
            InherentImplsDiagnostics::IncoherentType(impl_id) => sink.push(IncoherentImpl {
                impl_: impl_id
                    .lookup(db.upcast())
                    .source(db.upcast())
                    .as_ref()
                    .map(AstPtr::new),
            }),
            InherentImplsDiagnostics::DuplicateDefinitions(first, second) => {
                sink.push(DuplicateDefinition {
                    definition: assoc_item_syntax_node_ptr(db.upcast(), second),
                    first_definition: assoc_item_syntax_node_ptr(db.upcast(), first),
                    name: assoc_item_name(db.upcast(), first),
                });
            }
        }
    }

    fn module_id(&self, db: &dyn DefDatabase) -> ModuleId {
        match self {
            InherentImplsDiagnostics::LowerDiagnostic(impl_id, _)
            | InherentImplsDiagnostics::InvalidSelfTy(impl_id)
            | InherentImplsDiagnostics::IncoherentType(impl_id) => impl_id.module(db),
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

#[cfg(test)]
mod tests {
    use crate::{
        mock::MockDatabase, with_fixture::WithFixture, DiagnosticSink, HirDatabase, SourceDatabase,
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
}

mod collector;
#[cfg(test)]
mod tests;

use std::{ops::Index, sync::Arc};

use la_arena::ArenaMap;
use mun_hir_input::{ModuleTree, PackageId, PackageModuleId};

use crate::{item_scope::ItemScope, DefDatabase, DiagnosticSink};

/// Contains all top-level definitions for a package.
#[derive(Debug, PartialEq, Eq)]
pub struct PackageDefs {
    pub id: PackageId,
    pub modules: ArenaMap<PackageModuleId, ItemScope>,
    pub module_tree: Arc<ModuleTree>,
    diagnostics: Vec<diagnostics::DefDiagnostic>,
}

impl PackageDefs {
    /// Constructs a `PackageDefs` for the specified `package` with the data
    /// from the `db`.
    pub(crate) fn package_def_map_query(
        db: &dyn DefDatabase,
        package: PackageId,
    ) -> Arc<PackageDefs> {
        Arc::new(collector::collect(db, package))
    }

    /// Adds all the diagnostics for the specified `module` to the `sink`.
    pub fn add_diagnostics(
        &self,
        db: &dyn DefDatabase,
        module: PackageModuleId,
        sink: &mut DiagnosticSink<'_>,
    ) {
        for diagnostic in self.diagnostics.iter() {
            diagnostic.add_to(db, module, sink);
        }
    }
}

impl Index<PackageModuleId> for PackageDefs {
    type Output = ItemScope;

    fn index(&self, index: PackageModuleId) -> &Self::Output {
        &self.modules[index]
    }
}

mod diagnostics {
    use mun_hir_input::PackageModuleId;
    use mun_syntax::{ast, ast::Use, AstPtr};

    use crate::{
        diagnostics::{ImportDuplicateDefinition, UnresolvedImport},
        source_id::AstId,
        AstDatabase, DefDatabase, DiagnosticSink, InFile, Path,
    };

    /// A type of diagnostic that may be emitted during resolving all package
    /// definitions.
    #[derive(Debug, PartialEq, Eq)]
    enum DiagnosticKind {
        UnresolvedImport { ast: AstId<ast::Use>, index: usize },
        DuplicateImport { ast: AstId<ast::Use>, index: usize },
    }

    /// A diagnostic that may be emitted during resolving all package
    /// definitions.
    #[derive(Debug, PartialEq, Eq)]
    pub(super) struct DefDiagnostic {
        /// The module that contains the diagnostic
        in_module: PackageModuleId,

        /// The type of diagnostic
        kind: DiagnosticKind,
    }

    impl DefDiagnostic {
        /// Constructs a new `DefDiagnostic` which indicates that an import
        /// could not be resolved.
        pub(super) fn unresolved_import(
            container: PackageModuleId,
            ast: AstId<ast::Use>,
            index: usize,
        ) -> Self {
            Self {
                in_module: container,
                kind: DiagnosticKind::UnresolvedImport { ast, index },
            }
        }

        /// Constructs a new `DefDiagnostic` which indicates that an import
        /// names a duplication.
        pub(super) fn duplicate_import(
            container: PackageModuleId,
            ast: AstId<ast::Use>,
            index: usize,
        ) -> Self {
            Self {
                in_module: container,
                kind: DiagnosticKind::DuplicateImport { ast, index },
            }
        }

        pub(super) fn add_to(
            &self,
            db: &dyn DefDatabase,
            target_module: PackageModuleId,
            sink: &mut DiagnosticSink<'_>,
        ) {
            fn use_tree_ptr_from_ast(
                db: &dyn AstDatabase,
                ast: &AstId<Use>,
                index: usize,
            ) -> Option<InFile<AstPtr<ast::UseTree>>> {
                let use_item = ast.to_node(db);
                let mut cur = 0;
                let mut tree = None;
                Path::expand_use_item(&use_item, |_path, use_tree, _is_glob, _alias| {
                    if cur == index {
                        tree = Some(use_tree.clone());
                    }
                    cur += 1;
                });
                tree.map(|t| InFile::new(ast.file_id, AstPtr::new(&t)))
            }

            if self.in_module != target_module {
                return;
            }

            match &self.kind {
                DiagnosticKind::UnresolvedImport { ast, index } => {
                    if let Some(use_tree) = use_tree_ptr_from_ast(db, ast, *index) {
                        sink.push(UnresolvedImport { use_tree });
                    }
                }
                DiagnosticKind::DuplicateImport { ast, index } => {
                    if let Some(use_tree) = use_tree_ptr_from_ast(db, ast, *index) {
                        sink.push(ImportDuplicateDefinition { use_tree });
                    }
                }
            }
        }
    }
}

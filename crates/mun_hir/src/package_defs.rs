mod collector;

use crate::{
    arena::map::ArenaMap, item_scope::ItemScope, module_tree::LocalModuleId,
    module_tree::ModuleTree, DefDatabase, DiagnosticSink, PackageId,
};
use std::{ops::Index, sync::Arc};

/// Contains all top-level definitions for a package.
#[derive(Debug, PartialEq, Eq)]
pub struct PackageDefs {
    pub modules: ArenaMap<LocalModuleId, ItemScope>,
    pub module_tree: Arc<ModuleTree>,
    diagnostics: Vec<diagnostics::DefDiagnostic>,
}

impl PackageDefs {
    /// Constructs a `PackageDefs` for the specified `package` with the data from the `db`.
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
        module: LocalModuleId,
        sink: &mut DiagnosticSink,
    ) {
        for diagnostic in self.diagnostics.iter() {
            diagnostic.add_to(db, module, sink)
        }
    }
}

impl Index<LocalModuleId> for PackageDefs {
    type Output = ItemScope;

    fn index(&self, index: LocalModuleId) -> &Self::Output {
        &self.modules[index]
    }
}

mod diagnostics {
    use crate::diagnostics::UnresolvedImport;
    use crate::{
        module_tree::LocalModuleId, source_id::AstId, DefDatabase, DiagnosticSink, InFile, Path,
    };
    use mun_syntax::{ast, AstPtr};

    #[derive(Debug, PartialEq, Eq)]
    enum DiagnosticKind {
        UnresolvedImport { ast: AstId<ast::Use>, index: usize },
    }

    #[derive(Debug, PartialEq, Eq)]
    pub(super) struct DefDiagnostic {
        /// The module that contains the diagnostic
        in_module: LocalModuleId,

        /// The type of diagnostic
        kind: DiagnosticKind,
    }

    impl DefDiagnostic {
        /// Constructs a new `DefDiagnostic` which indicates that an import could not be resolved.
        pub(super) fn unresolved_import(
            container: LocalModuleId,
            ast: AstId<ast::Use>,
            index: usize,
        ) -> Self {
            Self {
                in_module: container,
                kind: DiagnosticKind::UnresolvedImport { ast, index },
            }
        }

        pub(super) fn add_to(
            &self,
            db: &dyn DefDatabase,
            target_module: LocalModuleId,
            sink: &mut DiagnosticSink,
        ) {
            if self.in_module != target_module {
                return;
            }

            match &self.kind {
                DiagnosticKind::UnresolvedImport { ast, index } => {
                    let use_item = ast.to_node(db.upcast());
                    let mut cur = 0;
                    let mut tree = None;
                    Path::expand_use_item(
                        InFile::new(ast.file_id, use_item),
                        |_path, use_tree, _is_glob, _alias| {
                            if cur == *index {
                                tree = Some(use_tree.clone())
                            }
                            cur += 1;
                        },
                    );

                    if let Some(use_tree) = tree {
                        sink.push(UnresolvedImport {
                            use_tree: InFile::new(ast.file_id, AstPtr::new(&use_tree)),
                        });
                    }
                }
            }
        }
    }
}

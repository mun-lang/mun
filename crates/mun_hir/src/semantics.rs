//! `Semantics` provides the means to get semantic information from syntax trees
//! that are not necessarily part of the compilation process. This is useful
//! when you want to extract information from a modified source file in the
//! context of the current state.
//!
//! Our compilation databases (e.g. `HirDatabase`) provides a lot of steps to go
//! from a syntax tree (as provided by the [`mun_syntax::ast`] module) to more
//! abstract representations of the source through the process of `lowering`.
//! However, for IDE purposes we often want to cut through all this and go from
//! source locations straight to lowered data structures and back. This is what
//! [`Semantics`] enables.

mod source_to_def;

use std::cell::RefCell;

use mun_hir_input::FileId;
use mun_syntax::{ast, AstNode, SyntaxNode, TextSize};
use rustc_hash::FxHashMap;
use smallvec::SmallVec;

use crate::{
    ids::{DefWithBodyId, ImplId, ItemDefinitionId},
    resolve::{self, HasResolver},
    semantics::source_to_def::{SourceToDefCache, SourceToDefContainer, SourceToDefContext},
    source_analyzer::SourceAnalyzer,
    HirDatabase, InFile, ModuleDef, Name, PatId, PerNs, Resolver, Ty, Visibility,
};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PathResolution {
    /// An item
    Def(ModuleDef),
    /// A local binding (only value namespace)
    Local(Local),
    SelfType(Impl),
}

/// The primary API to get semantic information, like types, from syntax trees.
/// Exposes the database it was created with through the `db` field.
pub struct Semantics<'db> {
    pub db: &'db dyn HirDatabase,

    /// Cache of root syntax nodes to their `FileId`
    source_file_to_file: RefCell<FxHashMap<SyntaxNode, FileId>>,

    /// A cache to map source locations to definitions
    source_to_definition_cache: RefCell<SourceToDefCache>,
}

impl<'db> Semantics<'db> {
    /// Constructs a new `Semantics` instance with the given database.
    pub fn new(db: &'db dyn HirDatabase) -> Self {
        Self {
            db,
            source_file_to_file: RefCell::default(),
            source_to_definition_cache: RefCell::default(),
        }
    }

    /// Returns the Concrete Syntax Tree for the file with the given `file_id`.
    pub fn parse(&self, file_id: FileId) -> ast::SourceFile {
        let tree = self.db.parse(file_id).tree();
        let mut cache = self.source_file_to_file.borrow_mut();
        cache.insert(tree.syntax().clone(), file_id);
        tree
    }

    /// Computes the `SemanticScope` at the given position in a CST.
    pub fn scope_at_offset(&self, node: &SyntaxNode, offset: TextSize) -> SemanticsScope<'db> {
        let analyzer = self.analyze_with_offset(node, offset);
        SemanticsScope {
            db: self.db,
            file_id: analyzer.file_id,
            resolver: analyzer.resolver,
        }
    }

    /// Returns the type of the given expression
    pub fn type_of_expr(&self, expr: &ast::Expr) -> Option<Ty> {
        self.analyze(expr.syntax()).type_of_expr(self.db, expr)
    }

    /// Returns the source analyzer for the given node.
    fn analyze(&self, node: &SyntaxNode) -> SourceAnalyzer {
        self.build_analyzer(node, None)
    }

    /// Constructs a `SourceAnalyzer` for the token at the given offset.
    fn analyze_with_offset(&self, node: &SyntaxNode, offset: TextSize) -> SourceAnalyzer {
        self.build_analyzer(node, Some(offset))
    }

    /// Internal function that constructs a `SourceAnalyzer` from the given
    /// `node` and optional `offset` in the file.
    fn build_analyzer(&self, node: &SyntaxNode, offset: Option<TextSize>) -> SourceAnalyzer {
        let node = self.find_file(node.clone());
        let node = node.as_ref();

        // Find the "lowest" container that contains the code
        let container = match self.with_source_to_def_context(|ctx| ctx.find_container(node)) {
            Some(it) => it,
            None => return SourceAnalyzer::new_for_resolver(Resolver::default(), node),
        };

        // Construct an analyzer for the given container
        let resolver = match container {
            SourceToDefContainer::DefWithBodyId(def) => {
                return SourceAnalyzer::new_for_body(self.db, def, node, offset)
            }
            SourceToDefContainer::ModuleId(id) => id.resolver(self.db.upcast()),
        };

        SourceAnalyzer::new_for_resolver(resolver, node)
    }

    /// Runs a function with a `SourceToDefContext` which can be used to cache
    /// definition queries.
    fn with_source_to_def_context<F: FnOnce(&mut SourceToDefContext<'_, '_>) -> T, T>(
        &self,
        f: F,
    ) -> T {
        let mut cache = self.source_to_definition_cache.borrow_mut();
        let mut context = SourceToDefContext {
            db: self.db,
            cache: &mut cache,
        };
        f(&mut context)
    }

    /// Returns the file that is associated with the given root CST node or
    /// `None` if no such association exists.
    fn lookup_file(&self, root_node: &SyntaxNode) -> Option<FileId> {
        let cache = self.source_file_to_file.borrow();
        cache.get(root_node).copied()
    }

    /// Decorates the specified node with the file that it is associated with.
    fn find_file(&self, node: SyntaxNode) -> InFile<SyntaxNode> {
        let root_node = find_root(&node);
        let file_id = self.lookup_file(&root_node).unwrap_or_else(|| {
            panic!(
                "\n\nFailed to lookup {:?} in this Semantics.\n\
                 Make sure to use only query nodes, derived from this instance of Semantics.\n\
                 root node:   {:?}\n\
                 known nodes: {}\n\n",
                node,
                root_node,
                self.source_file_to_file
                    .borrow()
                    .keys()
                    .map(|it| format!("{it:?}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        });
        InFile::new(file_id, node)
    }

    /// Resolves the specified `ast::Path`
    pub fn resolve_path(&self, path: &ast::Path) -> Option<PathResolution> {
        self.analyze(path.syntax()).resolve_path(self.db, path)
    }
}

/// Returns the root node of the specified node.
fn find_root(node: &SyntaxNode) -> SyntaxNode {
    node.ancestors().last().unwrap()
}

/// Represents the notion of a scope (set of possible names) at a particular
/// position in source.
pub struct SemanticsScope<'a> {
    pub db: &'a dyn HirDatabase,
    file_id: FileId,
    resolver: Resolver,
}

/// Represents an element in a scope
pub enum ScopeDef {
    ModuleDef(ModuleDef),
    ImplSelfType(Impl),
    Local(Local),
    Unknown,
}

impl ScopeDef {
    /// Returns all the `ScopeDef`s from a `PerNs`. Never returns duplicates.
    pub fn all_items(def: PerNs<(ItemDefinitionId, Visibility)>) -> SmallVec<[Self; 2]> {
        let mut items = SmallVec::new();
        match (def.take_types(), def.take_values()) {
            (Some(ty), None) => items.push(ScopeDef::ModuleDef(ty.0.into())),
            (None, Some(val)) => items.push(ScopeDef::ModuleDef(val.0.into())),
            (Some(ty), Some(val)) => {
                // Some things are returned as both a value and a type, such as a unit struct.
                items.push(ScopeDef::ModuleDef(ty.0.into()));
                if ty != val {
                    items.push(ScopeDef::ModuleDef(val.0.into()));
                }
            }
            (None, None) => {}
        };

        if items.is_empty() {
            items.push(ScopeDef::Unknown);
        }

        items
    }
}

/// An `impl` block
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Impl {
    pub(crate) id: ImplId,
}

impl From<ImplId> for Impl {
    fn from(id: ImplId) -> Self {
        Impl { id }
    }
}

impl Impl {
    pub fn self_ty(self, db: &dyn HirDatabase) -> Ty {
        db.type_for_impl_self(self.id)
    }
}

/// A local variable in a body
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Local {
    pub(crate) parent: DefWithBodyId,
    pub(crate) pat_id: PatId,
}

impl Local {
    /// Returns the type of this local
    pub fn ty(self, db: &dyn HirDatabase) -> Ty {
        let infer = db.infer(self.parent);
        infer[self.pat_id].clone()
    }
}

impl SemanticsScope<'_> {
    /// Call the `visit` function for every named item in the scope
    pub fn visit_all_names(&self, visit: &mut dyn FnMut(Name, ScopeDef)) {
        let resolver = &self.resolver;

        resolver.visit_all_names(self.db.upcast(), &mut |name, def| {
            let def = match def {
                resolve::ScopeDef::ImplSelfType(id) => ScopeDef::ImplSelfType(Impl { id }),
                resolve::ScopeDef::PerNs(it) => {
                    let items = ScopeDef::all_items(it);
                    for item in items {
                        visit(name.clone(), item);
                    }
                    return;
                }
                resolve::ScopeDef::Local(pat_id) => {
                    let parent = resolver
                        .body_owner()
                        .expect("found a local outside of a body");
                    ScopeDef::Local(Local { parent, pat_id })
                }
            };
            visit(name, def);
        });
    }
}

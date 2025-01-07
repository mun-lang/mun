#![allow(dead_code)]

mod analysis;

use mun_hir::{
    semantics::{PathResolution, Semantics, SemanticsScope},
    AstDatabase, Ty,
};
use mun_syntax::{ast, AstNode, SyntaxNode};
use ra_ap_text_edit::Indel;

use crate::{
    completion::context::analysis::{analyze, AnalysisResult},
    db::AnalysisDatabase,
    FilePosition,
};

/// A `CompletionContext` is created to figure out where exactly the cursor is.
pub(super) struct CompletionContext<'a> {
    pub sema: Semantics<'a>,
    pub scope: SemanticsScope<'a>,
    pub db: &'a AnalysisDatabase,
    // TODO: Add this when it is used
    //pub position: FilePosition,
}

/// Information about the identifier that we are currently completing.
#[derive(Debug)]
pub(super) enum CompletionAnalysis {
    NameRef(NameRefContext),
}

/// The identifier to complete is a name reference.
#[derive(Debug)]
pub(super) struct NameRefContext {
    /// `NameRef` syntax in the original file
    pub(super) name_ref: Option<ast::NameRef>,
    pub(super) kind: NameRefKind,
}

/// The kind of the `NameRef` we are completing.
#[derive(Debug)]
pub(super) enum NameRefKind {
    Path(PathCompletionContext),
    DotAccess(DotAccess),
}

/// Information about the field or method access we are completing.
#[derive(Debug)]
pub(crate) struct DotAccess {
    pub(crate) receiver: Option<ast::Expr>,
    pub(crate) receiver_ty: Option<Ty>,
}

/// The state of the path we are currently completing.
#[derive(Debug)]
pub(super) struct PathCompletionContext {
    /// The type of path we are completing.
    pub(super) kind: PathKind,

    /// The qualifier of the current path.
    pub(super) qualified: Qualified,

    /// Whether the qualifier comes from a use tree parent or not
    pub(super) use_tree_parent: bool,
}

#[derive(Debug)]
pub(super) enum Qualified {
    /// No path qualifier, this is a bare path, e.g. `foo`
    No,

    /// The path has a qualifier, e.g. `foo` in `foo::bar`
    With {
        /// The path that is being completed
        path: ast::Path,

        /// The resolution of the path that is being completed
        resolution: Option<PathResolution>,
    },

    /// The path has an absolute qualifier, e.g. `::foo`
    Absolute,
}

/// The kind of path we are completing right now.
#[derive(Debug)]
pub(super) enum PathKind {
    Expr(PathExprContext),
    Use,
    SourceFile,
}

#[derive(Debug)]
pub(super) struct PathExprContext {}

impl<'a> CompletionContext<'a> {
    /// Tries to construct a new `CompletionContext` with the given database and
    /// file position.
    pub fn new(
        db: &'a AnalysisDatabase,
        position: FilePosition,
    ) -> Option<(Self, CompletionAnalysis)> {
        let sema = Semantics::new(db);

        let original_file = sema.parse(position.file_id);

        // Insert a fake identifier to get a valid parse tree. This tree will be used to
        // determine context. The actual original_file will be used for
        // completion.
        let file_with_fake_ident = {
            let parse = db.parse(position.file_id);
            let edit = Indel::insert(position.offset, String::from("intellijRulezz"));
            parse.reparse(&edit).tree()
        };

        // Get the current token
        let original_token = original_file
            .syntax()
            .token_at_offset(position.offset)
            .left_biased()?;

        // Analyze the context of the completion request
        let AnalysisResult { analysis } = analyze(
            &sema,
            original_file.syntax().clone(),
            file_with_fake_ident.syntax().clone(),
            position.offset,
        )?;

        let scope = sema.scope_at_offset(&original_token.parent()?, position.offset);

        let context = Self {
            sema,
            scope,
            db,
            // TODO: add this when it is used
            //position,
        };

        Some((context, analysis))
    }
}

/// Attempts to find `node` inside `syntax` via `node`'s text range.
/// If the fake identifier has been inserted after this node or inside of this
/// node use the `_compensated` version instead.
fn find_opt_node_in_file<N: AstNode>(syntax: &SyntaxNode, node: Option<N>) -> Option<N> {
    find_node_in_file(syntax, &node?)
}

/// Attempts to find `node` inside `syntax` via `node`'s text range.
/// If the fake identifier has been inserted after this node or inside of this
/// node use the `_compensated` version instead.
fn find_node_in_file<N: AstNode>(syntax: &SyntaxNode, node: &N) -> Option<N> {
    let syntax_range = syntax.text_range();
    let range = node.syntax().text_range();
    let intersection = range.intersect(syntax_range)?;
    syntax
        .covering_element(intersection)
        .ancestors()
        .find_map(N::cast)
}

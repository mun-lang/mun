use crate::db::AnalysisDatabase;
use crate::FilePosition;
use mun_hir::semantics::{Semantics, SemanticsScope};
use mun_hir::AstDatabase;
use mun_syntax::{ast, utils::find_node_at_offset, AstNode, SyntaxNode, TextRange, TextSize};
use ra_ap_text_edit::Indel;

/// A `CompletionContext` is created to figure out where exactly the cursor is.
pub(super) struct CompletionContext<'a> {
    pub sema: Semantics<'a>,
    pub scope: SemanticsScope<'a>,
    pub db: &'a AnalysisDatabase,

    // TODO: Add this when it is used
    //pub position: FilePosition,
    /// True if the context is currently at a trivial path.
    pub is_trivial_path: bool,

    /// True if the context is currently on a parameter
    pub is_param: bool,

    /// True if we're at an ast::PathType
    pub is_path_type: bool,

    /// The receiver if this is a field or method access, i.e. writing something.$0
    pub dot_receiver: Option<ast::Expr>,
}

impl<'a> CompletionContext<'a> {
    /// Tries to construct a new `CompletionContext` with the given database and file position.
    pub fn new(db: &'a AnalysisDatabase, position: FilePosition) -> Option<Self> {
        let sema = Semantics::new(db);

        let original_file = sema.parse(position.file_id);

        // Insert a fake identifier to get a valid parse tree. This tree will be used to determine
        // context. The actual original_file will be used for completion.
        let file_with_fake_ident = {
            let parse = db.parse(position.file_id);
            let edit = Indel::insert(position.offset, String::from("intellijRulezz"));
            parse.reparse(&edit).tree()
        };

        // Get the current token
        let token = original_file
            .syntax()
            .token_at_offset(position.offset)
            .left_biased()?;

        let scope = sema.scope_at_offset(&token.parent()?, position.offset);

        let mut context = Self {
            sema,
            scope,
            db,
            // TODO: add this when it is used
            //position,
            is_trivial_path: false,
            is_param: false,
            is_path_type: false,
            dot_receiver: None,
        };

        context.fill(
            &original_file.syntax().clone(),
            file_with_fake_ident.syntax().clone(),
            position.offset,
        );

        Some(context)
    }

    /// Examine the AST and determine what the context is at the given offset.
    fn fill(
        &mut self,
        original_file: &SyntaxNode,
        file_with_fake_ident: SyntaxNode,
        offset: TextSize,
    ) {
        // First, let's try to complete a reference to some declaration.
        if let Some(name_ref) = find_node_at_offset::<ast::NameRef>(&file_with_fake_ident, offset) {
            if is_node::<ast::Param>(name_ref.syntax()) {
                self.is_param = true;
                return;
            }

            self.classify_name_ref(original_file, name_ref);
        }
    }

    /// Classifies an `ast::NameRef`
    fn classify_name_ref(&mut self, original_file: &SyntaxNode, name_ref: ast::NameRef) {
        let parent = match name_ref.syntax().parent() {
            Some(it) => it,
            None => return,
        };

        // Complete references to declarations
        if let Some(segment) = ast::PathSegment::cast(parent.clone()) {
            let path = segment.parent_path();

            self.is_path_type = path
                .syntax()
                .parent()
                .and_then(ast::PathType::cast)
                .is_some();

            if let Some(segment) = path.segment() {
                if segment.has_colon_colon() {
                    return;
                }
            }

            self.is_trivial_path = true;
        }

        // Complete field expressions
        if let Some(field_expr) = ast::FieldExpr::cast(parent) {
            // The receiver comes before the point of insertion of the fake
            // ident, so it should have the same range in the non-modified file
            self.dot_receiver = field_expr
                .expr()
                .map(|e| e.syntax().text_range())
                .and_then(|r| find_node_with_range(original_file, r));
        }
    }
}

/// Returns true if the given `node` or one if its parents is of the specified type.
fn is_node<N: AstNode>(node: &SyntaxNode) -> bool {
    match node.ancestors().find_map(N::cast) {
        None => false,
        Some(n) => n.syntax().text_range() == node.text_range(),
    }
}

/// Returns a node that covers the specified range.
fn find_node_with_range<N: AstNode>(syntax: &SyntaxNode, range: TextRange) -> Option<N> {
    syntax.covering_element(range).ancestors().find_map(N::cast)
}

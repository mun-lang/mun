use std::sync::Arc;

use mun_syntax::{ast, AstNode, SyntaxNode, TextRange, TextSize};

use crate::{
    expr::{scope::LocalScopeId, BodySourceMap},
    ids::DefWithBodyId,
    resolver_for_scope, Body, ExprId, ExprScopes, FileId, HirDatabase, InFile, InferenceResult,
    Resolver, Ty,
};

/// A `SourceAnalyzer` is a wrapper which exposes the HIR API in terms of the
/// original source file. It's useful to query things from the syntax.
pub(crate) struct SourceAnalyzer {
    /// The file for which this analyzer was constructed
    pub(crate) file_id: FileId,

    /// The resolver used to resolve names
    pub(crate) resolver: Resolver,

    /// Optional body to res
    body: Option<Arc<Body>>,
    body_source_map: Option<Arc<BodySourceMap>>,
    infer: Option<Arc<InferenceResult>>,
    scopes: Option<Arc<ExprScopes>>,
}

impl SourceAnalyzer {
    /// Constructs a new `SourceAnalyzer` for the given `def` and with an
    /// optional offset in the source file.
    pub(crate) fn new_for_body(
        db: &dyn HirDatabase,
        def: DefWithBodyId,
        node: InFile<&SyntaxNode>,
        offset: Option<TextSize>,
    ) -> Self {
        let (body, source_map) = db.body_with_source_map(def);
        let scopes = db.expr_scopes(def);
        let scope = match offset {
            None => scope_for(&scopes, &source_map, node),
            Some(offset) => scope_for_offset(db, &scopes, &source_map, node.with_value(offset)),
        };
        let resolver = resolver_for_scope(db.upcast(), def, scope);
        SourceAnalyzer {
            resolver,
            body: Some(body),
            body_source_map: Some(source_map),
            infer: Some(db.infer(def)),
            scopes: Some(scopes),
            file_id: node.file_id,
        }
    }

    /// Constructs a new `SourceAnalyzer` from the specified `resolver`.
    pub(crate) fn new_for_resolver(
        resolver: Resolver,
        node: InFile<&SyntaxNode>,
    ) -> SourceAnalyzer {
        SourceAnalyzer {
            resolver,
            body: None,
            body_source_map: None,
            infer: None,
            scopes: None,
            file_id: node.file_id,
        }
    }

    /// Returns the type of the specified expression
    pub(crate) fn type_of_expr(&self, db: &dyn HirDatabase, expr: &ast::Expr) -> Option<Ty> {
        let expr_id = self.expr_id(db, expr)?;
        Some(self.infer.as_ref()?[expr_id].clone())
    }

    /// Returns the expression id of the given expression or None if it could
    /// not be found.
    fn expr_id(&self, _db: &dyn HirDatabase, expr: &ast::Expr) -> Option<ExprId> {
        let sm = self.body_source_map.as_ref()?;
        sm.node_expr(expr)
    }
}

/// Returns the id of the scope that is active at the location of `node`.
fn scope_for(
    scopes: &ExprScopes,
    source_map: &BodySourceMap,
    node: InFile<&SyntaxNode>,
) -> Option<LocalScopeId> {
    node.value
        .ancestors()
        .filter_map(ast::Expr::cast)
        .filter_map(|it| source_map.node_expr(&it))
        .find_map(|it| scopes.scope_for(it))
}

/// Computes the id of the scope that is closest to the given `offset`.
fn scope_for_offset(
    db: &dyn HirDatabase,
    scopes: &ExprScopes,
    source_map: &BodySourceMap,
    offset: InFile<TextSize>,
) -> Option<LocalScopeId> {
    // Get all scopes and their ranges
    let scopes_and_ranges = scopes.scope_by_expr().iter().filter_map(|(id, scope)| {
        let source = source_map.expr_syntax(*id)?;
        // FIXME: correctly handle macro expansion
        if source.file_id != offset.file_id {
            return None;
        }
        let root = source.file_syntax(db.upcast());
        let node = source
            .value
            .either(|ptr| ptr.syntax_node_ptr(), |ptr| ptr.syntax_node_ptr());
        Some((node.to_node(&root).text_range(), scope))
    });

    let smallest_scope_containing_offset = scopes_and_ranges.min_by_key(|(expr_range, _scope)| {
        (
            !(expr_range.start() <= offset.value && offset.value <= expr_range.end()),
            expr_range.len(),
        )
    });

    smallest_scope_containing_offset.map(|(expr_range, scope)| {
        adjust(db, scopes, source_map, expr_range, offset).unwrap_or(*scope)
    })
}

/// During completion the cursor may be outside of any expression. Given the
/// range of the containing scope, finds the scope that is most likely the scope
/// that the user is requesting.
fn adjust(
    db: &dyn HirDatabase,
    scopes: &ExprScopes,
    source_map: &BodySourceMap,
    expr_range: TextRange,
    offset: InFile<TextSize>,
) -> Option<LocalScopeId> {
    let child_scopes = scopes
        .scope_by_expr()
        .iter()
        .filter_map(|(id, scope)| {
            let source = source_map.expr_syntax(*id)?;
            if source.file_id != offset.file_id {
                return None;
            }
            let root = source.file_syntax(db.upcast());
            let node = source
                .value
                .either(|ptr| ptr.syntax_node_ptr(), |ptr| ptr.syntax_node_ptr());
            Some((node.to_node(&root).text_range(), scope))
        })
        .filter(|&(range, _)| {
            // The start of the scope is before the offset
            range.start() <= offset.value
            // The range is contained inside the expression scope
            && expr_range.contains_range(range)
            // The range is not the expression scope itself
            && range != expr_range
        });

    child_scopes
        .into_iter()
        .max_by(|&(r1, _), &(r2, _)| {
            if r1.contains_range(r2) {
                std::cmp::Ordering::Greater
            } else if r2.contains_range(r1) {
                std::cmp::Ordering::Less
            } else {
                r1.start().cmp(&r2.start())
            }
        })
        .map(|(_ptr, scope)| *scope)
}

use mun_hir::semantics::Semantics;
use mun_syntax::{ast, match_ast, utils::find_node_at_offset, AstNode, SyntaxNode, TextSize};

use super::{
    find_node_in_file, find_opt_node_in_file, CompletionAnalysis, DotAccess, NameRefContext,
    NameRefKind, PathCompletionContext, PathExprContext, PathKind, Qualified,
};

/// The result of the analysis of a completion request. This contains
/// information about the context of the completion request which helps identify
/// the surrounding code and the position of the cursor.
pub(super) struct AnalysisResult {
    pub(super) analysis: CompletionAnalysis,
}

pub fn analyze(
    sema: &Semantics<'_>,
    original_file: SyntaxNode,
    speculative_file: SyntaxNode,
    offset: TextSize,
) -> Option<AnalysisResult> {
    if let Some(name_ref) = find_node_at_offset::<ast::NameRef>(&speculative_file, offset) {
        let parent = name_ref.syntax().parent()?;
        let name_ref_ctx = classify_name_ref(sema, &original_file, name_ref, parent)?;
        return Some(AnalysisResult {
            analysis: CompletionAnalysis::NameRef(name_ref_ctx),
        });
    }

    None
}

fn classify_name_ref(
    sema: &Semantics<'_>,
    original_file: &SyntaxNode,
    name_ref: ast::NameRef,
    parent: SyntaxNode,
) -> Option<NameRefContext> {
    let name_ref = find_node_at_offset(original_file, name_ref.syntax().text_range().start());

    let segment = match_ast! {
        match parent {
            ast::PathSegment(segment) => segment,
            ast::FieldExpr(field) => {
                let receiver = find_opt_node_in_file(original_file, field.expr());
                let kind = NameRefKind::DotAccess(DotAccess {
                    receiver_ty: receiver.as_ref().and_then(|it| sema.type_of_expr(it)),
                    receiver
                });
                return Some(NameRefContext {
                    name_ref,
                    kind,
                });
            },
            _ => return None,
        }
    };

    let path = segment.parent_path();

    let mut path_ctx = PathCompletionContext {
        qualified: Qualified::No,
        use_tree_parent: false,
        kind: PathKind::SourceFile,
    };

    let make_path_kind_expr = |_expr: ast::Expr| PathKind::Expr(PathExprContext {});

    // Infer the type of path
    let parent = path.syntax().parent()?;
    let kind = match_ast! {
        match parent {
            ast::PathExpr(it) => {
                make_path_kind_expr(it.into())
            },
            ast::UseTree(_) => PathKind::Use,
            _ => return None,
        }
    };

    path_ctx.kind = kind;

    // If the path has a qualifier, we need to determine if it is a use tree or a
    // path
    if let Some((qualifier, use_tree_parent)) = path_or_use_tree_qualifier(&path) {
        path_ctx.use_tree_parent = use_tree_parent;
        if !use_tree_parent && segment.has_colon_colon() {
            path_ctx.qualified = Qualified::Absolute;
        } else {
            let qualifier = qualifier
                .segment()
                .and_then(|it| find_node_in_file(original_file, &it))
                .map(|it| it.parent_path());
            if let Some(qualifier) = qualifier {
                let res = sema.resolve_path(&qualifier);
                path_ctx.qualified = Qualified::With {
                    path: qualifier,
                    resolution: res,
                }
            }
        }
    } else if let Some(segment) = path.segment() {
        if segment.has_colon_colon() {
            path_ctx.qualified = Qualified::Absolute;
        }
    }

    Some(NameRefContext {
        name_ref,
        kind: NameRefKind::Path(path_ctx),
    })
}

fn path_or_use_tree_qualifier(path: &ast::Path) -> Option<(ast::Path, bool)> {
    if let Some(qual) = path.qualifier() {
        return Some((qual, false));
    }
    let use_tree_list = path.syntax().ancestors().find_map(ast::UseTreeList::cast)?;
    let use_tree = use_tree_list
        .syntax()
        .parent()
        .and_then(ast::UseTree::cast)?;
    Some((use_tree.path()?, true))
}

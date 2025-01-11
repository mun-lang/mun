use super::{dot, expr, CompletionContext, Completions, NameRefContext, NameRefKind, PathKind};

/// Generate completions for a name reference.
#[allow(clippy::single_match)]
pub(super) fn complete_name_ref(
    completions: &mut Completions,
    ctx: &CompletionContext<'_>,
    NameRefContext { kind, .. }: &NameRefContext,
) {
    match kind {
        NameRefKind::Path(path_ctx) => match &path_ctx.kind {
            PathKind::Expr(expr_ctx) => {
                expr::complete_expr_path(completions, ctx, path_ctx, expr_ctx);
            }
            _ => {}
        },
        NameRefKind::DotAccess(dot_access) => {
            dot::complete_dot(completions, ctx, dot_access);
        }
    }
}

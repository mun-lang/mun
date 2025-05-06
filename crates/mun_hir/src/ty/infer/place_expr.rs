use crate::{resolve::ValueNs, ty::infer::InferenceResultBuilder, Expr, ExprId, Path, Resolver};

impl InferenceResultBuilder<'_> {
    /// Checks if the specified expression is a place-expression. A place
    /// expression represents a memory location.
    pub(super) fn check_place_expression(&mut self, resolver: &Resolver, expr: ExprId) -> bool {
        match &self.body[expr] {
            Expr::Path(p) => self.check_place_path(resolver, p),
            Expr::Index { base, .. } => self.check_place_expression(resolver, *base),
            Expr::Field { .. } | Expr::Array(_) => true,
            _ => false,
        }
    }

    /// Checks if the specified path references a memory location.
    fn check_place_path(&mut self, resolver: &Resolver, path: &Path) -> bool {
        match resolver.resolve_path_as_value_fully(self.db, path) {
            Some((ValueNs::ImplSelf(_) | ValueNs::LocalBinding(_), _)) => true,
            Some((ValueNs::FunctionId(_) | ValueNs::StructId(_) | ValueNs::ConstId(_), _))
            | None => false,
        }
    }
}

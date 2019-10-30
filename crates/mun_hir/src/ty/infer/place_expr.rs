use crate::{
    ty::infer::InferenceResultBuilder, Expr, ExprId, HirDatabase, Path, Resolution, Resolver,
};
use std::sync::Arc;

impl<'a, D: HirDatabase> InferenceResultBuilder<'a, D> {
    /// Checks if the specified expression is a place-expression. A place expression represents a
    /// memory location.
    pub(super) fn check_place_expression(&mut self, resolver: &Resolver, expr: ExprId) -> bool {
        let body = Arc::clone(&self.body); // avoid borrow checker problem
        match &body[expr] {
            Expr::Path(p) => self.check_place_path(resolver, p),
            _ => false,
        }
    }

    /// Checks if the specified path references a memory location.
    fn check_place_path(&mut self, resolver: &Resolver, path: &Path) -> bool {
        let resolution = match resolver
            .resolve_path_without_assoc_items(self.db, path)
            .take_values()
        {
            Some(resolution) => resolution,
            None => return false,
        };

        match resolution {
            Resolution::LocalBinding(_) => true,
            Resolution::Def(_) => false,
        }
    }
}

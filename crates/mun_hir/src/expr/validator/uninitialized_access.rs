use super::ExprValidator;
use crate::HirDatabase;
//use crate::{HirDatabase, ExprId, PatId, Expr, Path};
//use std::collections::HashSet;

//enum ExprSide {
//    left,
//    Expr
//}

impl<'a, 'b, 'd, D: HirDatabase> ExprValidator<'a, 'b, 'd, D> {
    /// Validates that all binding access has previously been initialized.
    pub(super) fn validate_uninitialized_access(&mut self) {
        //        let mut initialized_patterns = HashSet::new();
        //
        //        // Add all parameter patterns to the set of initialized patterns (they must have been
        //        // initialized)
        //        for (pat, _) in self.body.params.iter() {
        //            initialized_patterns.insert(*pat)
        //        }
        //
        //        self.validate_expr_access(&mut initialized_patterns, self.body.body_expr);
    }

    //    /// Validates that the specified expr does not access unitialized bindings
    //    fn validate_expr_access(&mut self, initialized_patterns: &mut HashSet<PatId>, expr: ExprId, exprSide:ExprSide) {
    //        let body = self.body.clone();
    //        match &body[expr] {
    //            Expr::Call { callee, args } => {
    //                self.validate_expr_access(initialized_patterns, *callee);
    //                for arg in args.iter() {
    //                    self.validate_expr_access(initialized_patterns, *callee);
    //                }
    //            },
    //            Expr::Path(p) => {
    //                let resolver = expr::resolver_for_expr(self.body.clone(), self.db, tgt_expr);
    //                self.validate_path_access(initialized_patterns, &resolver, p);
    //            }
    //            Expr::If { .. } => {},
    //            Expr::UnaryOp { .. } => {},
    //            Expr::BinaryOp { .. } => {},
    //            Expr::Block { .. } => {},
    //            Expr::Return { .. } => {},
    //            Expr::Break { .. } => {},
    //            Expr::Loop { .. } => {},
    //            Expr::While { .. } => {},
    //            Expr::RecordLit { .. } => {},
    //            Expr::Field { .. } => {},
    //            Expr::Literal(_) => {},
    //            Expr::Missing => {},
    //        }
    //    }
    //
    //    fn validate_expr_access(&mut self, initialized_patterns: &mut HashSet<PatId>, p: &Path) {
    //
    //    }
}

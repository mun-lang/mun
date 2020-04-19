use super::ExprValidator;
use crate::diagnostics::{DiagnosticSink, LiteralOutOfRange};
use crate::ty::ResolveBitness;
use crate::{ty_app, TypeCtor};
use crate::{Expr, HirDatabase, HirDisplay, Literal};

impl<'d, D: HirDatabase> ExprValidator<'d, D> {
    /// Iterates over all expressions to determine if one of the literals has a value that is out of
    /// range of its type.
    pub fn validate_literal_ranges(&self, sink: &mut DiagnosticSink) {
        self.body[self.body.body_expr].walk_child_exprs(move |expr_id| {
            let expr = &self.body[expr_id];
            if let Expr::Literal(Literal::Int(lit)) = &expr {
                let ty = &self.infer[expr_id];
                match ty {
                    ty_app!(TypeCtor::Int(int_ty)) => {
                        if lit.value > int_ty.resolve(&self.db.target_data_layout()).max() {
                            let literal = self
                                .body_source_map
                                .expr_syntax(expr_id)
                                .expect("could not retrieve expr from source map")
                                .map(|expr_src| {
                                    expr_src
                                        .left()
                                        .expect("could not retrieve expr from ExprSource")
                                        .cast()
                                        .expect("could not cast expression to literal")
                                });
                            sink.push(LiteralOutOfRange {
                                literal,
                                int_ty: *int_ty,
                            })
                        }
                    }
                    _ => panic!(
                        "expected int literal to have int ty while instead it is `{}`",
                        ty.display(self.db)
                    ),
                }
            }
        })
    }
}

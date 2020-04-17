use super::ExprValidator;
use crate::diagnostics::{DiagnosticSink, PossiblyUninitializedVariable};
use crate::{BinaryOp, Expr, ExprId, HirDatabase, PatId, Path, Resolution, Resolver, Statement};
use std::collections::HashSet;

#[derive(Copy, Clone, PartialEq, Eq)]
enum ExprKind {
    Normal,
    Place,
    Both,
}

impl<'d, D: HirDatabase> ExprValidator<'d, D> {
    /// Validates that all binding access has previously been initialized.
    pub(super) fn validate_uninitialized_access(&self, sink: &mut DiagnosticSink) {
        let mut initialized_patterns = HashSet::new();

        // Add all parameter patterns to the set of initialized patterns (they must have been
        // initialized)
        for (pat, _) in self.body.params.iter() {
            initialized_patterns.insert(*pat);
        }

        self.validate_expr_access(
            sink,
            &mut initialized_patterns,
            self.body.body_expr,
            ExprKind::Normal,
        );
    }

    /// Validates that the specified expr does not access unitialized bindings
    fn validate_expr_access(
        &self,
        sink: &mut DiagnosticSink,
        initialized_patterns: &mut HashSet<PatId>,
        expr: ExprId,
        expr_side: ExprKind,
    ) {
        let body = self.body.clone();
        match &body[expr] {
            Expr::Call { callee, args } => {
                self.validate_expr_access(sink, initialized_patterns, *callee, expr_side);
                for arg in args.iter() {
                    self.validate_expr_access(sink, initialized_patterns, *arg, expr_side);
                }
            }
            Expr::Path(p) => {
                let resolver = crate::expr::resolver_for_expr(self.body.clone(), self.db, expr);
                self.validate_path_access(
                    sink,
                    initialized_patterns,
                    &resolver,
                    p,
                    expr,
                    expr_side,
                );
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.validate_expr_access(sink, initialized_patterns, *condition, ExprKind::Normal);
                let mut then_branch_initialized_patterns = initialized_patterns.clone();
                self.validate_expr_access(
                    sink,
                    &mut then_branch_initialized_patterns,
                    *then_branch,
                    ExprKind::Normal,
                );
                if let Some(else_branch) = else_branch {
                    let mut else_branch_initialized_patterns = initialized_patterns.clone();
                    self.validate_expr_access(
                        sink,
                        &mut else_branch_initialized_patterns,
                        *else_branch,
                        ExprKind::Normal,
                    );
                    let then_is_never = self.infer[*then_branch].is_never();
                    let else_is_never = self.infer[*else_branch].is_never();
                    match (then_is_never, else_is_never) {
                        (false, false) => {
                            initialized_patterns.extend(
                                then_branch_initialized_patterns
                                    .intersection(&else_branch_initialized_patterns),
                            );
                        }
                        (true, false) => {
                            initialized_patterns.extend(else_branch_initialized_patterns);
                        }
                        (false, true) => {
                            initialized_patterns.extend(then_branch_initialized_patterns);
                        }
                        (true, true) => {}
                    };
                }
            }
            Expr::UnaryOp { expr, .. } => {
                self.validate_expr_access(sink, initialized_patterns, *expr, ExprKind::Normal);
            }
            Expr::BinaryOp { lhs, rhs, op } => {
                let lhs_expr_kind = match op {
                    Some(BinaryOp::Assignment { op: Some(_) }) => ExprKind::Both,
                    Some(BinaryOp::Assignment { op: None }) => ExprKind::Place,
                    _ => ExprKind::Normal,
                };
                self.validate_expr_access(sink, initialized_patterns, *lhs, lhs_expr_kind);
                self.validate_expr_access(sink, initialized_patterns, *rhs, ExprKind::Normal)
            }
            Expr::Block { statements, tail } => {
                for statement in statements.iter() {
                    match statement {
                        Statement::Let {
                            pat, initializer, ..
                        } => {
                            if let Some(initializer) = initializer {
                                self.validate_expr_access(
                                    sink,
                                    initialized_patterns,
                                    *initializer,
                                    ExprKind::Normal,
                                );
                                initialized_patterns.insert(*pat);
                            }
                        }
                        Statement::Expr(expr) => {
                            self.validate_expr_access(
                                sink,
                                initialized_patterns,
                                *expr,
                                ExprKind::Normal,
                            );
                            if self.infer[*expr].is_never() {
                                return;
                            }
                        }
                    }
                }
                if let Some(tail) = tail {
                    self.validate_expr_access(sink, initialized_patterns, *tail, ExprKind::Normal)
                }
            }
            Expr::Return { expr } => {
                if let Some(expr) = expr {
                    self.validate_expr_access(sink, initialized_patterns, *expr, ExprKind::Normal)
                }
            }
            Expr::Break { expr } => {
                if let Some(expr) = expr {
                    self.validate_expr_access(sink, initialized_patterns, *expr, ExprKind::Normal)
                }
            }
            Expr::Loop { body } => {
                self.validate_expr_access(sink, initialized_patterns, *body, ExprKind::Normal)
            }
            Expr::While { condition, body } => {
                self.validate_expr_access(sink, initialized_patterns, *condition, ExprKind::Normal);
                self.validate_expr_access(
                    sink,
                    &mut initialized_patterns.clone(),
                    *body,
                    ExprKind::Normal,
                );
            }
            Expr::RecordLit { fields, spread, .. } => {
                for field in fields.iter() {
                    self.validate_expr_access(
                        sink,
                        initialized_patterns,
                        field.expr,
                        ExprKind::Normal,
                    );
                }
                if let Some(expr) = spread {
                    self.validate_expr_access(sink, initialized_patterns, *expr, ExprKind::Normal);
                }
            }
            Expr::Field { expr, .. } => {
                self.validate_expr_access(sink, initialized_patterns, *expr, ExprKind::Normal);
            }
            Expr::Literal(_) => {}
            Expr::Missing => {}
        }
    }

    fn validate_path_access(
        &self,
        sink: &mut DiagnosticSink,
        initialized_patterns: &mut HashSet<PatId>,
        resolver: &Resolver,
        path: &Path,
        expr: ExprId,
        expr_side: ExprKind,
    ) {
        let resolution = match resolver
            .resolve_path_without_assoc_items(self.db, path)
            .take_values()
        {
            Some(resolution) => resolution,
            None => {
                // This has already been caught by the inferencing step
                return;
            }
        };

        let pat = match resolution {
            Resolution::LocalBinding(pat) => pat,
            Resolution::Def(_def) => return,
        };

        if expr_side == ExprKind::Normal || expr_side == ExprKind::Both {
            // Check if the binding has already been initialized
            if initialized_patterns.get(&pat).is_none() {
                let (_, body_source_map) = self.db.body_with_source_map(self.func.into());
                sink.push(PossiblyUninitializedVariable {
                    file: self.func.module(self.db).file_id(),
                    pat: body_source_map
                        .expr_syntax(expr)
                        .unwrap()
                        .value
                        .either(|it| it.syntax_node_ptr(), |it| it.syntax_node_ptr()),
                })
            }
        }

        if expr_side == ExprKind::Place {
            // The binding should be initialized
            initialized_patterns.insert(pat);
        }
    }
}

use crate::code_model::src::HasSource;
use crate::diagnostics::{ExternCannotHaveBody, ExternNonPrimitiveParam};
use crate::expr::BodySourceMap;
use crate::{diagnostics::DiagnosticSink, Body, Expr, Function, HirDatabase, InferenceResult};
use mun_syntax::{AstNode, SyntaxNodePtr};
use std::sync::Arc;

mod uninitialized_access;

#[cfg(test)]
mod tests;

pub struct ExprValidator<'a, 'b: 'a, 'd, DB: HirDatabase> {
    func: Function,
    infer: Arc<InferenceResult>,
    body: Arc<Body>,
    body_source_map: Arc<BodySourceMap>,
    sink: &'a mut DiagnosticSink<'b>,
    db: &'d DB,
}

impl<'a, 'b, 'd, DB: HirDatabase> ExprValidator<'a, 'b, 'd, DB> {
    pub fn new(func: Function, db: &'d DB, sink: &'a mut DiagnosticSink<'b>) -> Self {
        let (body, body_source_map) = db.body_with_source_map(func.into());
        ExprValidator {
            func,
            sink,
            db,
            infer: db.infer(func.into()),
            body,
            body_source_map,
        }
    }

    pub fn validate_body(&mut self) {
        self.validate_uninitialized_access();
        self.validate_extern();
    }
    pub fn validate_extern(&mut self) {
        if !self.func.is_extern(self.db) {
            return;
        }

        // Validate that there is not body
        match self.body[self.func.body(self.db).body_expr] {
            Expr::Missing => {}
            _ => self.sink.push(ExternCannotHaveBody {
                func: self
                    .func
                    .source(self.db)
                    .map(|f| SyntaxNodePtr::new(f.syntax())),
            }),
        }

        // Validate the arguments
        for (arg, _) in &self.body.params {
            let param_id = &self.infer[*arg];
            if param_id.as_struct().is_some() {
                self.sink.push(ExternNonPrimitiveParam {
                    param: self
                        .body_source_map
                        .pat_syntax(*arg)
                        .unwrap()
                        .map(|arg| arg.syntax_node_ptr()),
                })
            }
        }
    }
}

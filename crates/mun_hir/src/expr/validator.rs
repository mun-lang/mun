use crate::{diagnostics::DiagnosticSink, Body, Function, HirDatabase, InferenceResult};
use std::sync::Arc;

mod uninitialized_access;

pub struct ExprValidator<'a, 'b: 'a, 'd, DB: HirDatabase> {
    func: Function,
    infer: Arc<InferenceResult>,
    body: Arc<Body>,
    sink: &'a mut DiagnosticSink<'b>,
    db: &'d DB,
}

impl<'a, 'b, 'd, DB: HirDatabase> ExprValidator<'a, 'b, 'd, DB> {
    pub fn new(func: Function, db: &'d DB, sink: &'a mut DiagnosticSink<'b>) -> Self {
        ExprValidator {
            func,
            sink,
            db,
            infer: db.infer(func.into()),
            body: db.body(func.into()),
        }
    }

    pub fn validate_body(&mut self) {
        self.validate_uninitialized_access();
    }
}

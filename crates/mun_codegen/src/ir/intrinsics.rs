use crate::intrinsics::{self, Intrinsic};
use crate::ir::dispatch_table::FunctionPrototype;
use crate::IrDatabase;
use hir::{Body, Expr, ExprId, InferenceResult};
use inkwell::module::Module;
use inkwell::types::FunctionType;
use std::collections::BTreeMap;
use std::sync::Arc;

// Use a `BTreeMap` to guarantee deterministically ordered output
pub type IntrinsicsMap = BTreeMap<FunctionPrototype, FunctionType>;

fn collect_intrinsic(module: &Module, entries: &mut IntrinsicsMap, intrinsic: &impl Intrinsic) {
    let prototype = intrinsic.prototype();
    entries
        .entry(prototype)
        .or_insert_with(|| intrinsic.ir_type(&module.get_context()));
}

fn collect_expr<D: IrDatabase>(
    db: &D,
    module: &Module,
    entries: &mut IntrinsicsMap,
    expr_id: ExprId,
    body: &Arc<Body>,
    infer: &InferenceResult,
) {
    let expr = &body[expr_id];

    // If this expression is a call, store it in the dispatch table
    if let Expr::Call { callee, .. } = expr {
        match infer[*callee].as_callable_def() {
            Some(hir::CallableDef::Struct(_)) => {
                collect_intrinsic(module, entries, &intrinsics::new);
                collect_intrinsic(module, entries, &intrinsics::clone);
                // self.collect_intrinsic(module, entries, &intrinsics::drop);
            }
            Some(hir::CallableDef::Function(_)) => (),
            None => panic!("expected a callable expression"),
        }
    }

    if let Expr::RecordLit { .. } = expr {
        collect_intrinsic(module, entries, &intrinsics::new);
        collect_intrinsic(module, entries, &intrinsics::clone);
        // self.collect_intrinsic(module, entries, &intrinsics::drop);
    }

    if let Expr::Path(path) = expr {
        let resolver = hir::resolver_for_expr(body.clone(), db, expr_id);
        let resolution = resolver
            .resolve_path_without_assoc_items(db, path)
            .take_values()
            .expect("unknown path");

        if let hir::Resolution::Def(hir::ModuleDef::Struct(_)) = resolution {
            collect_intrinsic(module, entries, &intrinsics::new);
            collect_intrinsic(module, entries, &intrinsics::clone);
            // self.collect_intrinsic( module, entries, &intrinsics::drop);
        }
    }

    // Recurse further
    expr.walk_child_exprs(|expr_id| collect_expr(db, module, entries, expr_id, body, infer))
}

pub fn collect_fn_body<D: IrDatabase>(
    db: &D,
    module: &Module,
    entries: &mut IntrinsicsMap,
    body: &Arc<Body>,
    infer: &InferenceResult,
) {
    collect_expr(db, module, entries, body.body_expr(), body, infer);
}

pub fn collect_wrapper_body(module: &Module, entries: &mut IntrinsicsMap) {
    collect_intrinsic(module, entries, &intrinsics::new);
    collect_intrinsic(module, entries, &intrinsics::clone);
    // self.collect_intrinsic(entries, &intrinsics::drop, module);
}

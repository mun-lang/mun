use crate::code_gen::CodeGenConfig;
use inkwell::context::Context;
use crate::intrinsics::{self, Intrinsic};
use crate::ir::dispatch_table::FunctionPrototype;
use hir::{Body, Expr, ExprId, InferenceResult};
use inkwell::types::FunctionType;
use std::collections::BTreeMap;
use std::sync::Arc;

// Use a `BTreeMap` to guarantee deterministically ordered output
pub type IntrinsicsMap<'ink> = BTreeMap<FunctionPrototype, FunctionType<'ink>>;

fn collect_intrinsic<'ink>(
    context: &'ink Context,
    config: &CodeGenConfig,
    entries: &mut IntrinsicsMap<'ink>,
    intrinsic: &impl Intrinsic<'ink>,
) {
    let prototype = intrinsic.prototype(context, &config.target_data);
    entries
        .entry(prototype)
        .or_insert_with(|| intrinsic.ir_type(context, &config.target_data));
}

fn collect_expr<'ink, D: hir::HirDatabase>(
    context: &'ink Context,
    config: &CodeGenConfig,
    db: &D,
    entries: &mut IntrinsicsMap<'ink>,
    needs_alloc: &mut bool,
    expr_id: ExprId,
    body: &Arc<Body>,
    infer: &InferenceResult,
) {
    let expr = &body[expr_id];

    // If this expression is a call, store it in the dispatch table
    if let Expr::Call { callee, .. } = expr {
        match infer[*callee].as_callable_def() {
            Some(hir::CallableDef::Struct(_)) => {
                collect_intrinsic(context, config, entries, &intrinsics::new);
                // self.collect_intrinsic(module, entries, &intrinsics::drop);
                *needs_alloc = true;
            }
            Some(hir::CallableDef::Function(_)) => (),
            None => panic!("expected a callable expression"),
        }
    }

    if let Expr::RecordLit { .. } = expr {
        collect_intrinsic(context, config, entries, &intrinsics::new);
        // self.collect_intrinsic(module, entries, &intrinsics::drop);
        *needs_alloc = true;
    }

    if let Expr::Path(path) = expr {
        let resolver = hir::resolver_for_expr(body.clone(), db, expr_id);
        let resolution = resolver
            .resolve_path_without_assoc_items(db, path)
            .take_values()
            .expect("unknown path");

        if let hir::Resolution::Def(hir::ModuleDef::Struct(_)) = resolution {
            collect_intrinsic(context, config, entries, &intrinsics::new);
            // self.collect_intrinsic( module, entries, &intrinsics::drop);
            *needs_alloc = true;
        }
    }

    // Recurse further
    expr.walk_child_exprs(|expr_id| collect_expr(context, config, db, entries, needs_alloc, expr_id, body, infer))
}

pub fn collect_fn_body<'ink, D: hir::HirDatabase>(
    context: &'ink Context,
    config: &CodeGenConfig,
    db: &D,
    entries: &mut IntrinsicsMap<'ink>,
    needs_alloc: &mut bool,
    body: &Arc<Body>,
    infer: &InferenceResult,
) {
    collect_expr(context, config, db, entries, needs_alloc, body.body_expr(), body, infer);
}

pub fn collect_wrapper_body<'ink>(
    context: &'ink Context,
    config: &CodeGenConfig,
    entries: &mut IntrinsicsMap<'ink>,
    needs_alloc: &mut bool,
) {
    collect_intrinsic(context, config, entries, &intrinsics::new);
    // self.collect_intrinsic(entries, &intrinsics::drop, module);
    *needs_alloc = true;
}

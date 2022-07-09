use crate::{
    intrinsics::{self, Intrinsic},
    ir::dispatch_table::FunctionPrototype,
};
use hir::{Body, Expr, ExprId, HirDatabase, InferenceResult, ValueNs};
use inkwell::{context::Context, targets::TargetData, types::FunctionType};
use std::{collections::BTreeMap, sync::Arc};

// Use a `BTreeMap` to guarantee deterministically ordered output
pub type IntrinsicsMap<'ink> = BTreeMap<FunctionPrototype, FunctionType<'ink>>;

/// Stores the type information from the `intrinsic` in `entries`
fn collect_intrinsic<'ink>(
    context: &'ink Context,
    target: &TargetData,
    intrinsic: &impl Intrinsic,
    entries: &mut IntrinsicsMap<'ink>,
) {
    let prototype = intrinsic.prototype();
    entries
        .entry(prototype)
        .or_insert_with(|| intrinsic.ir_type(context, target));
}

/// Iterates over all expressions and stores information on which intrinsics they use in `entries`.
#[allow(clippy::too_many_arguments)]
fn collect_expr<'db, 'ink>(
    context: &'ink Context,
    target: &TargetData,
    db: &'db dyn HirDatabase,
    intrinsics: &mut IntrinsicsMap<'ink>,
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
                collect_intrinsic(context, target, &intrinsics::new, intrinsics);
                // self.collect_intrinsic(module, entries, &intrinsics::drop);
                *needs_alloc = true;
            }
            Some(hir::CallableDef::Function(_)) => (),
            None => panic!("expected a callable expression"),
        }
    }

    if let Expr::RecordLit { .. } = expr {
        collect_intrinsic(context, target, &intrinsics::new, intrinsics);
        // self.collect_intrinsic(module, entries, &intrinsics::drop);
        *needs_alloc = true;
    }

    if let Expr::Path(path) = expr {
        let resolver = hir::resolver_for_expr(db.upcast(), body.owner(), expr_id);
        if let Some((ValueNs::StructId(_), _)) =
            resolver.resolve_path_as_value_fully(db.upcast(), path)
        {
            collect_intrinsic(context, target, &intrinsics::new, intrinsics);
            // self.collect_intrinsic( module, entries, &intrinsics::drop);
            *needs_alloc = true;
        }
    }

    if let Expr::Array(_) = expr {
        collect_intrinsic(context, target, &intrinsics::new_array, intrinsics);
        *needs_alloc = true;
    }

    // Recurse further
    expr.walk_child_exprs(|expr_id| {
        collect_expr(
            context,
            target,
            db,
            intrinsics,
            needs_alloc,
            expr_id,
            body,
            infer,
        )
    })
}

/// Collects all intrinsics from the specified `body`.
pub fn collect_fn_body<'db, 'ink>(
    context: &'ink Context,
    target: TargetData,
    db: &'db dyn HirDatabase,
    intrinsics: &mut IntrinsicsMap<'ink>,
    needs_alloc: &mut bool,
    body: &Arc<Body>,
    infer: &InferenceResult,
) {
    collect_expr(
        context,
        &target,
        db,
        intrinsics,
        needs_alloc,
        body.body_expr(),
        body,
        infer,
    );
}

/// Collects all intrinsics from a function wrapper body.
pub fn collect_wrapper_body<'ink>(
    context: &'ink Context,
    target: TargetData,
    intrinsics: &mut IntrinsicsMap<'ink>,
    needs_alloc: &mut bool,
) {
    collect_intrinsic(context, &target, &intrinsics::new, intrinsics);
    // self.collect_intrinsic(entries, &intrinsics::drop, module);
    *needs_alloc = true;
}

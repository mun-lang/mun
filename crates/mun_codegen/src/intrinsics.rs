use std::ffi;

use std::{collections::BTreeMap, sync::Arc};

use mun_hir::{Body, Expr, ExprId, HirDatabase, InferenceResult, ValueNs};

use crate::{dispatch_table::FunctionPrototype, intrinsics};

#[macro_use]
mod macros;

// Use a `BTreeMap` to guarantee deterministically ordered output
pub type IntrinsicsSet = BTreeMap<FunctionPrototype, mun_hir::FnSig>;

/// Defines the properties of an intrinsic function that can be called from Mun.
/// These functions are mostly used internally.
pub trait Intrinsic: Sync {
    /// Returns the function signature as seen by type inference.
    fn callable_sig(&self) -> mun_hir::FnSig;

    /// Returns the prototype of the intrinsic
    fn prototype(&self) -> FunctionPrototype;
}

impl_intrinsics! {
    /// Allocates memory for the specified `type` in the allocator referred to by `alloc_handle`.
    pub fn new(type_handle: *const ffi::c_void, alloc_handle: *mut ffi::c_void) -> *const *mut ffi::c_void;

    /// Allocates memory for an array of the specified `type` in the allocator referred to by
    /// `alloc_handle` with at least enough capacity to hold `length` elements.
    ///
    /// Note that the elements in the array are left uninitialized.
    pub fn new_array(type_handle: *const ffi::c_void, length: usize, alloc_handle: *mut ffi::c_void) -> *const *mut ffi::c_void;
}

/// Stores the type information from the `intrinsic` in `entries`
fn collect_intrinsic(intrinsic: &impl Intrinsic, entries: &mut IntrinsicsSet) {
    let prototype = intrinsic.prototype();
    entries
        .entry(prototype)
        .or_insert_with(|| intrinsic.callable_sig());
}

/// Iterates over all expressions and stores information on which intrinsics
/// they use in `entries`.
fn collect_expr(
    db: &'_ dyn HirDatabase,
    intrinsics: &mut IntrinsicsSet,
    needs_alloc: &mut bool,
    expr_id: ExprId,
    body: &Arc<Body>,
    infer: &InferenceResult,
) {
    let expr = &body[expr_id];

    // If this expression is a call, store it in the dispatch table
    if let Expr::Call { callee, .. } = expr {
        match infer[*callee].as_callable_def() {
            Some(mun_hir::CallableDef::Struct(_)) => {
                collect_intrinsic(&intrinsics::new, intrinsics);
                // self.collect_intrinsic(module, entries, &intrinsics::drop);
                *needs_alloc = true;
            }
            Some(mun_hir::CallableDef::Function(_)) => (),
            None => panic!("expected a callable expression"),
        }
    }

    if let Expr::RecordLit { .. } = expr {
        collect_intrinsic(&intrinsics::new, intrinsics);
        // self.collect_intrinsic(module, entries, &intrinsics::drop);
        *needs_alloc = true;
    }

    if let Expr::Path(path) = expr {
        let resolver = mun_hir::resolver_for_expr(db.upcast(), body.owner(), expr_id);
        if let Some((ValueNs::StructId(_), _)) =
            resolver.resolve_path_as_value_fully(db.upcast(), path)
        {
            collect_intrinsic(&intrinsics::new, intrinsics);
            // self.collect_intrinsic( module, entries, &intrinsics::drop);
            *needs_alloc = true;
        }
    }

    if let Expr::Array(_) = expr {
        collect_intrinsic(&intrinsics::new_array, intrinsics);
        *needs_alloc = true;
    }

    // Recurse further
    expr.walk_child_exprs(|expr_id| {
        collect_expr(db, intrinsics, needs_alloc, expr_id, body, infer);
    });
}

/// Collects all intrinsics from the specified `body`.
pub fn collect_fn_body(
    db: &dyn HirDatabase,
    intrinsics: &mut IntrinsicsSet,
    needs_alloc: &mut bool,
    body: &Arc<Body>,
    infer: &InferenceResult,
) {
    collect_expr(db, intrinsics, needs_alloc, body.body_expr(), body, infer);
}

/// Collects all intrinsics from a function wrapper body.
pub fn collect_wrapper_body(intrinsics: &mut IntrinsicsSet, needs_alloc: &mut bool) {
    collect_intrinsic(&intrinsics::new, intrinsics);
    // self.collect_intrinsic(entries, &intrinsics::drop, module);
    *needs_alloc = true;
}

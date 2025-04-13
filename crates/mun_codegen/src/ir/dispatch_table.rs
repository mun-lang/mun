use std::{collections::HashMap, sync::Arc};

use either::Either;
use mun_hir::{Body, Expr, ExprId, HirDatabase, InferenceResult};
use rustc_hash::FxHashSet;

use crate::{
    intrinsics::Intrinsic,
    ir::ty::HirTypeCache,
    module_group::ModuleGroup,
    type_info::{HasStaticTypeId, TypeId},
};

use super::intrinsics::IntrinsicsSet;

/// A dispatch table in IR is a struct that contains pointers to all functions
/// that are called from code. In C terms it looks something like this:
/// ```c
/// struct DispatchTable {
///     int(*foo)(int, int);
///     // .. etc
/// } dispatchTable;
/// ```
///
/// The dispatch table is used to add a patchable indirection when calling a
/// function from IR. The `DispatchTable` is exposed to the Runtime which fills
/// the structure with valid pointers to functions. This basically enables all
/// hot reloading within Mun.
#[derive(Debug, Eq, PartialEq)]
pub struct DispatchTable {
    // This contains the function that map to the DispatchTable struct fields
    function_to_idx: HashMap<mun_hir::Function, usize>,
    // Prototype to function index
    prototype_to_idx: HashMap<FunctionPrototype, usize>,
    // This contains an ordered list of all the function in the dispatch table
    entries: Vec<DispatchableFunction>,
}

/// A `FunctionPrototype` defines a unique signature that can be added to the
/// dispatch table.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct FunctionPrototype {
    pub name: String,
    pub arg_types: Vec<Arc<TypeId>>,
    pub ret_type: Arc<TypeId>,
}

/// A `DispatchableFunction` is an entry in the dispatch table that may or may
/// not be pointing to an existing `mun_hir` function.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DispatchableFunction {
    pub prototype: FunctionPrototype,
    pub mun_hir: Either<mun_hir::Function, mun_hir::FnSig>,
}

impl DispatchTable {
    /// Returns whether the `DispatchTable` contains the specified `function`.
    pub fn contains(&self, function: mun_hir::Function) -> bool {
        self.function_to_idx.contains_key(&function)
    }

    /// Returns a slice containing all the functions in the dispatch table.
    pub fn entries(&self) -> &[DispatchableFunction] {
        &self.entries
    }

    /// Finds the index of the provided function in the dispatch table, if it exists.
    pub fn index_by_function(&self, function: mun_hir::Function) -> Option<usize> {
        self.function_to_idx.get(&function).copied()
    }

    /// Finds the index of the provided intrinsic in the dispatch table, if it exists.
    pub fn index_by_intrinsic(&self, intrinsic: &impl Intrinsic) -> Option<usize> {
        let prototype = intrinsic.prototype();

        self.prototype_to_idx.get(&prototype).copied()
    }
}

/// A struct that can be used to build the dispatch table from HIR.
pub(crate) struct DispatchTableBuilder<'db, 'group> {
    db: &'db dyn HirDatabase,
    // Converts HIR ty's to `TypeId`s.
    hir_types: &'group HirTypeCache<'db>,
    // This contains the functions that map to the DispatchTable struct fields
    function_to_idx: HashMap<mun_hir::Function, usize>,
    // Prototype to function index
    prototype_to_idx: HashMap<FunctionPrototype, usize>,
    // These are *all* called functions in the modules
    entries: Vec<DispatchableFunction>,
    // The group of modules for which the dispatch table is being build
    module_group: &'group ModuleGroup,
    // The set of modules that is referenced
    referenced_modules: FxHashSet<mun_hir::Module>,
}

impl<'db, 'group> DispatchTableBuilder<'db, 'group> {
    /// Creates a new builder that can generate a dispatch function.
    pub fn new(
        db: &'db dyn HirDatabase,
        intrinsics: &IntrinsicsSet,
        hir_types: &'group HirTypeCache<'db>,
        module_group: &'group ModuleGroup,
    ) -> Self {
        let mut table = Self {
            db,
            hir_types,
            function_to_idx: HashMap::default(),
            prototype_to_idx: HashMap::default(),
            entries: Vec::default(),
            module_group,
            referenced_modules: FxHashSet::default(),
        };

        if !intrinsics.is_empty() {
            // Use a `BTreeSet` to guarantee deterministically ordered output
            for (prototype, fn_sig) in intrinsics.iter() {
                let index = table.entries.len();
                table.entries.push(DispatchableFunction {
                    prototype: prototype.clone(),
                    mun_hir: Either::Right(fn_sig.clone()),
                });

                table.prototype_to_idx.insert(prototype.clone(), index);
            }
        }
        table
    }

    /// Collects call expression from the given expression and sub expressions.
    fn collect_expr(&mut self, expr_id: ExprId, body: &Arc<Body>, infer: &InferenceResult) {
        let expr = &body[expr_id];

        // If this expression is a call, store it in the dispatch table
        if let Expr::Call { callee, .. } = expr {
            match infer[*callee].as_callable_def() {
                Some(mun_hir::CallableDef::Function(def)) => {
                    if self.module_group.should_runtime_link_fn(self.db, def) {
                        let fn_module = def.module(self.db);
                        if !def.is_extern(self.db) && !self.module_group.contains(fn_module) {
                            self.referenced_modules.insert(fn_module);
                        }
                        self.collect_fn_def(def);
                    }
                }
                Some(mun_hir::CallableDef::Struct(_)) => (),
                None => panic!("expected a callable expression"),
            }
        }

        // Recurse further
        expr.walk_child_exprs(|expr_id| self.collect_expr(expr_id, body, infer));
    }

    /// Collects function call expression from the given expression.
    #[allow(clippy::map_entry)]
    pub fn collect_fn_def(&mut self, function: mun_hir::Function) {
        // If the function is not yet contained in the table, add it
        if !self.function_to_idx.contains_key(&function) {
            let name = function.full_name(self.db);
            let hir_type = function.ty(self.db);
            let sig = hir_type.callable_sig(self.db).unwrap();
            let arg_types = sig
                .params()
                .iter()
                .map(|arg| self.hir_types.type_id(arg))
                .collect();
            let ret_type = if sig.ret().is_empty() {
                <()>::type_id().clone()
            } else {
                self.hir_types.type_id(sig.ret())
            };

            let prototype = FunctionPrototype {
                name,
                arg_types,
                ret_type,
            };
            let index = self.entries.len();
            self.entries.push(DispatchableFunction {
                prototype: prototype.clone(),
                mun_hir: Either::Left(function),
            });
            self.prototype_to_idx.insert(prototype, index);
            self.function_to_idx.insert(function, index);
        }
    }

    /// Collect all the call expressions from the specified body with the given
    /// type inference result.
    pub fn collect_body(&mut self, body: &Arc<Body>, infer: &InferenceResult) {
        self.collect_expr(body.body_expr(), body, infer);
    }

    /// Builds the final `DispatchTable` with all *called* functions from within
    /// the module.
    ///
    /// Returns the `DispatchTable` and a set of dependencies for the module.
    pub fn build(self) -> (DispatchTable, FxHashSet<mun_hir::Module>) {
        (
            DispatchTable {
                function_to_idx: self.function_to_idx,
                prototype_to_idx: self.prototype_to_idx,
                entries: self.entries,
            },
            self.referenced_modules,
        )
    }
}

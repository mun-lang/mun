use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use mun_hir::{Body, ExprId, HirDatabase, InferenceResult};

use crate::{
    ir::{dispatch_table::DispatchTable, ty::HirTypeCache},
    type_info::TypeId,
    ModuleGroup,
};

/// A type table in IR is a list of pointers to unique type information that are
/// used to generate function and struct information.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TypeTable {
    entries: Vec<Arc<TypeId>>,
    type_id_to_index: HashMap<Arc<TypeId>, usize>,
}

impl TypeTable {
    /// Returns a slice containing all types
    pub fn entries(&self) -> &[Arc<TypeId>] {
        &self.entries
    }

    /// Generates a `TypeInfo` lookup through the `TypeTable`, equivalent to
    /// something along the lines of: `type_table[i]`, where `i` is the
    /// index of the type and `type_table` is an array of `TypeInfo`
    /// pointers.
    pub fn index_of_type(&self, type_id: &Arc<TypeId>) -> Option<usize> {
        self.type_id_to_index.get(type_id).cloned()
    }

    /// Returns whether the type table is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Used to build a `TypeTable` from HIR.
pub(crate) struct TypeTableBuilder<'db, 't> {
    db: &'db dyn HirDatabase,
    dispatch_table: &'t DispatchTable,
    hir_types: &'t HirTypeCache<'db>,
    entries: HashSet<Arc<TypeId>>,
    module_group: &'t ModuleGroup,
}

impl<'db, 't> TypeTableBuilder<'db, 't> {
    /// Creates a new `TypeTableBuilder`.
    pub(crate) fn new(
        db: &'db dyn HirDatabase,
        dispatch_table: &'t DispatchTable,
        hir_types: &'t HirTypeCache<'db>,
        module_group: &'t ModuleGroup,
    ) -> Self {
        Self {
            db,
            dispatch_table,
            hir_types,
            entries: HashSet::default(),
            module_group,
        }
    }

    /// Collects unique `TypeInfo` from the given `Ty`.
    fn collect_type(&mut self, type_info: Arc<TypeId>) {
        self.entries.insert(type_info);
    }

    /// Collects unique `TypeInfo` from the specified expression and its
    /// sub-expressions.
    fn collect_expr(&mut self, expr_id: ExprId, body: &Arc<Body>, infer: &InferenceResult) {
        let expr = &body[expr_id];

        // If this expression is a call, store it in the dispatch table
        if let mun_hir::Expr::Call { callee, .. } = expr {
            match infer[*callee].as_callable_def() {
                Some(mun_hir::CallableDef::Function(hir_fn)) => {
                    self.maybe_collect_fn_signature(hir_fn);
                }
                Some(mun_hir::CallableDef::Struct(_)) => (),
                None => panic!("expected a callable expression"),
            }
        } else if let mun_hir::Expr::Array(..) = expr {
            self.collect_type(self.hir_types.type_id(&infer[expr_id]));
        }

        // Recurse further
        expr.walk_child_exprs(|expr_id| self.collect_expr(expr_id, body, infer));
    }

    /// Collects `TypeInfo` from types in the signature of a function
    pub fn collect_fn_signature(&mut self, hir_fn: mun_hir::Function) {
        let fn_sig = hir_fn.ty(self.db).callable_sig(self.db).unwrap();

        // Collect argument types
        for ty in fn_sig.params().iter() {
            self.collect_type(self.hir_types.type_id(ty));
        }

        // Collect return type
        let ret_ty = fn_sig.ret();
        if !ret_ty.is_empty() {
            self.collect_type(self.hir_types.type_id(ret_ty));
        }
    }

    /// Collects `TypeInfo` from types in the signature of a function if it's
    /// exposed externally.
    pub fn maybe_collect_fn_signature(&mut self, hir_fn: mun_hir::Function) {
        // If a function is externally visible or contained in the dispatch table,
        // record the types of the signature
        if self.module_group.should_export_fn(self.db, hir_fn)
            || self.dispatch_table.contains(hir_fn)
        {
            self.collect_fn_signature(hir_fn);
        }
    }

    /// Collects unique `TypeInfo` from the specified function signature and
    /// body.
    pub fn collect_fn(&mut self, hir_fn: mun_hir::Function) {
        self.maybe_collect_fn_signature(hir_fn);

        // Collect used types from body
        let body = hir_fn.body(self.db);
        let infer = hir_fn.infer(self.db);
        self.collect_expr(body.body_expr(), &body, &infer);
    }

    /// Collects unique `TypeInfo` from the specified struct type.
    pub fn collect_struct(&mut self, hir_struct: mun_hir::Struct) {
        let type_info = self.hir_types.type_id(&hir_struct.ty(self.db));
        self.collect_type(type_info);

        let fields = hir_struct.fields(self.db);
        for field in fields {
            self.collect_type(self.hir_types.type_id(&field.ty(self.db)));
        }
    }

    /// Constructs a `TypeTable` from all *used* types.
    pub fn build(self) -> TypeTable {
        let mut entries = Vec::from_iter(self.entries);
        entries.sort_by(|a, b| a.name.cmp(&b.name));

        let type_info_to_index = entries
            .iter()
            .enumerate()
            .map(|(idx, type_info)| (type_info.clone(), idx))
            .collect();

        TypeTable {
            entries,
            type_id_to_index: type_info_to_index,
        }
    }
}

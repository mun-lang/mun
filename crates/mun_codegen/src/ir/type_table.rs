use std::{
    collections::{HashMap, HashSet},
    convert::TryInto,
    sync::Arc,
};

use inkwell::{
    context::Context,
    module::{Linkage, Module},
    types::ArrayType,
    values::{GlobalValue, PointerValue},
    AddressSpace,
};
use mun_hir::{Body, ExprId, HirDatabase, InferenceResult};

use crate::{
    ir::{
        dispatch_table::{DispatchTable, FunctionPrototype},
        ty::HirTypeCache,
    },
    type_info::TypeId,
    ModuleGroup,
};

/// The runtime lookup table for type handles referenced by generated code.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TypeTable<'ink> {
    entries: Vec<Arc<TypeId>>,
    type_id_to_index: HashMap<Arc<TypeId>, usize>,
    table_type: ArrayType<'ink>,
}

impl<'ink> TypeTable<'ink> {
    pub(crate) const NAME: &'static str = "global_type_lookup_table";

    pub fn entries(&self) -> &[Arc<TypeId>] {
        &self.entries
    }

    pub fn find_global(module: &Module<'ink>) -> Option<GlobalValue<'ink>> {
        module.get_global(Self::NAME)
    }

    /// Emits a lookup for the runtime handle associated with `type_info`.
    pub fn gen_type_info_lookup(
        &self,
        context: &'ink Context,
        builder: &inkwell::builder::Builder<'ink>,
        type_info: &Arc<TypeId>,
        table_ref: Option<GlobalValue<'ink>>,
    ) -> PointerValue<'ink> {
        let table_ref = table_ref.expect("no type table defined");
        let index: u64 = (*self.type_id_to_index.get(type_info).expect("unknown type"))
            .try_into()
            .expect("too many types");
        let global_index = context.i64_type().const_zero();
        let array_index = context.i64_type().const_int(index, false);
        let pointer = unsafe {
            builder.build_gep(
                table_ref.as_pointer_value(),
                &[global_index, array_index],
                &format!("{}_ptr_ptr", type_info.name),
            )
        };
        builder
            .build_load(pointer, &format!("{}_ptr", type_info.name))
            .into_pointer_value()
    }

    pub fn num_types(&self) -> usize {
        self.table_type.len() as usize
    }

    pub fn is_empty(&self) -> bool {
        self.table_type.len() == 0
    }

    pub fn ty(&self) -> ArrayType<'ink> {
        self.table_type
    }
}

/// Collects the types used by a module group and materializes its runtime lookup table.
pub(crate) struct TypeTableBuilder<'db, 'ink, 't> {
    db: &'db dyn HirDatabase,
    context: &'ink Context,
    module: &'t Module<'ink>,
    dispatch_table: &'t DispatchTable<'ink>,
    hir_types: &'t HirTypeCache<'db, 'ink>,
    entries: HashSet<Arc<TypeId>>,
    module_group: &'t ModuleGroup,
}

impl<'db, 'ink, 't> TypeTableBuilder<'db, 'ink, 't> {
    pub(crate) fn new<'f>(
        db: &'db dyn HirDatabase,
        context: &'ink Context,
        module: &'t Module<'ink>,
        _intrinsics: impl Iterator<Item = &'f FunctionPrototype>,
        dispatch_table: &'t DispatchTable<'ink>,
        hir_types: &'t HirTypeCache<'db, 'ink>,
        module_group: &'t ModuleGroup,
    ) -> Self {
        Self {
            db,
            context,
            module,
            dispatch_table,
            hir_types,
            entries: HashSet::default(),
            module_group,
        }
    }

    fn collect_type(&mut self, type_info: Arc<TypeId>) {
        self.entries.insert(type_info);
    }

    fn collect_expr(&mut self, expr_id: ExprId, body: &Arc<Body>, infer: &InferenceResult) {
        let expr = &body[expr_id];
        if let mun_hir::Expr::Call { callee, .. } = expr {
            match infer[*callee].as_callable_def() {
                Some(mun_hir::CallableDef::Function(function)) => {
                    self.maybe_collect_fn_signature(function);
                }
                Some(mun_hir::CallableDef::Struct(_)) => (),
                None => panic!("expected a callable expression"),
            }
        } else if let mun_hir::Expr::Array(..) = expr {
            self.collect_type(self.hir_types.type_id(&infer[expr_id]));
        }
        expr.walk_child_exprs(|expr_id| self.collect_expr(expr_id, body, infer));
    }

    pub fn collect_fn_signature(&mut self, function: mun_hir::Function) {
        let signature = function.ty(self.db).callable_sig(self.db).unwrap();
        for ty in signature.params().iter() {
            self.collect_type(self.hir_types.type_id(ty));
        }
        if !signature.ret().is_empty() {
            self.collect_type(self.hir_types.type_id(signature.ret()));
        }
    }

    pub fn maybe_collect_fn_signature(&mut self, function: mun_hir::Function) {
        if self.module_group.should_export_fn(self.db, function)
            || self.dispatch_table.contains(function)
        {
            self.collect_fn_signature(function);
        }
    }

    pub fn collect_fn(&mut self, function: mun_hir::Function) {
        self.maybe_collect_fn_signature(function);
        let body = function.body(self.db);
        let infer = function.infer(self.db);
        self.collect_expr(body.body_expr(), &body, &infer);
    }

    pub fn collect_struct(&mut self, strukt: mun_hir::Struct) {
        self.collect_type(self.hir_types.type_id(&strukt.ty(self.db)));
        for field in strukt.fields(self.db) {
            self.collect_type(self.hir_types.type_id(&field.ty(self.db)));
        }
    }

    pub fn build(self) -> TypeTable<'ink> {
        let mut entries = Vec::from_iter(self.entries);
        entries.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));
        let type_id_to_index = entries
            .iter()
            .enumerate()
            .map(|(index, type_info)| (type_info.clone(), index))
            .collect();

        let pointer_type = self.context.i8_type().ptr_type(AddressSpace::default());
        let values = vec![pointer_type.const_null(); entries.len()];
        let initializer = pointer_type.const_array(&values);
        if !values.is_empty() {
            let global = self
                .module
                .add_global(initializer.get_type(), None, TypeTable::NAME);
            global.set_linkage(Linkage::External);
            global.set_initializer(&initializer);
        }

        TypeTable {
            entries,
            type_id_to_index,
            table_type: initializer.get_type(),
        }
    }
}

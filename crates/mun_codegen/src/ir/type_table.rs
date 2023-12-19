use std::{collections::HashMap, collections::HashSet, convert::TryInto, sync::Arc};

use inkwell::{
    context::Context, module::Linkage, module::Module, types::ArrayType, values::PointerValue,
};

use mun_hir::{Body, ExprId, HirDatabase, InferenceResult};

use crate::{
    ir::dispatch_table::{DispatchTable, FunctionPrototype},
    ir::ty::HirTypeCache,
    type_info::TypeId,
    value::{Global, IrValueContext, IterAsIrValue, Value},
    ModuleGroup,
};

/// A type table in IR is a list of pointers to unique type information that are used to generate
/// function and struct information.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TypeTable<'ink> {
    entries: Vec<Arc<TypeId>>,
    type_id_to_index: HashMap<Arc<TypeId>, usize>,
    table_type: ArrayType<'ink>,
}

impl<'ink> TypeTable<'ink> {
    /// The name of the TypeTable's LLVM `GlobalValue`.
    pub(crate) const NAME: &'static str = "global_type_lookup_table";

    /// Returns a slice containing all types
    pub fn entries(&self) -> &[Arc<TypeId>] {
        &self.entries
    }

    /// Looks for a global symbol with the name of the TypeTable global in the specified `module`.
    /// Returns the global value if it could be found, `None` otherwise.
    pub fn find_global(module: &Module<'ink>) -> Option<Global<'ink, [*const std::ffi::c_void]>> {
        module
            .get_global(Self::NAME)
            .map(|g| unsafe { Global::from_raw(g) })
    }

    /// Generates a `TypeInfo` lookup through the `TypeTable`, equivalent to something along the
    /// lines of: `type_table[i]`, where `i` is the index of the type and `type_table` is an array
    /// of `TypeInfo` pointers.
    pub fn gen_type_info_lookup(
        &self,
        context: &'ink Context,
        builder: &inkwell::builder::Builder<'ink>,
        type_info: &Arc<TypeId>,
        table_ref: Option<Global<'ink, [*const std::ffi::c_void]>>,
    ) -> PointerValue<'ink> {
        let table_ref = table_ref.expect("no type table defined");

        let index: u64 = (*self.type_id_to_index.get(type_info).expect("unknown type"))
            .try_into()
            .expect("too many types");

        let global_index = context.i64_type().const_zero();
        let array_index = context.i64_type().const_int(index, false);

        let ptr_to_type_info_ptr = unsafe {
            builder.build_gep(
                table_ref.into(),
                &[global_index, array_index],
                &format!("{}_ptr_ptr", type_info.name),
            )
        };

        builder
            .build_load(ptr_to_type_info_ptr, &format!("{}_ptr", type_info.name))
            .into_pointer_value()
    }

    /// Returns the number of types in the `TypeTable`.
    pub fn num_types(&self) -> usize {
        self.table_type.len() as usize
    }

    /// Returns whether the type table is empty.
    pub fn is_empty(&self) -> bool {
        self.table_type.len() == 0
    }

    /// Returns the IR type of the type table's global value, if it exists.
    pub fn ty(&self) -> ArrayType<'ink> {
        self.table_type
    }
}

/// Used to build a `TypeTable` from HIR.
pub(crate) struct TypeTableBuilder<'db, 'ink, 't> {
    db: &'db dyn HirDatabase,
    value_context: &'t IrValueContext<'ink, 't, 't>,
    dispatch_table: &'t DispatchTable<'ink>,
    hir_types: &'t HirTypeCache<'db, 'ink>,
    entries: HashSet<Arc<TypeId>>,
    module_group: &'t ModuleGroup,
}

impl<'db, 'ink, 't> TypeTableBuilder<'db, 'ink, 't> {
    /// Creates a new `TypeTableBuilder`.
    pub(crate) fn new<'f>(
        db: &'db dyn HirDatabase,
        value_context: &'t IrValueContext<'ink, '_, '_>,
        _intrinsics: impl Iterator<Item = &'f FunctionPrototype>,
        dispatch_table: &'t DispatchTable<'ink>,
        hir_types: &'t HirTypeCache<'db, 'ink>,
        module_group: &'t ModuleGroup,
    ) -> Self {
        // for prototype in intrinsics {
        //     for arg_type in prototype.arg_types.iter() {
        //         builder.collect_type(arg_type.clone());
        //     }
        //     if let Some(ret_type) = prototype.ret_type.as_ref() {
        //         builder.collect_type(ret_type.clone());
        //     }
        // }

        Self {
            db,
            value_context,
            dispatch_table,
            hir_types,
            entries: Default::default(),
            module_group,
        }
    }

    /// Collects unique `TypeInfo` from the given `Ty`.
    fn collect_type(&mut self, type_info: Arc<TypeId>) {
        self.entries.insert(type_info);
    }

    /// Collects unique `TypeInfo` from the specified expression and its sub-expressions.
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
            self.collect_type(self.hir_types.type_id(&infer[expr_id]))
        }

        // Recurse further
        expr.walk_child_exprs(|expr_id| self.collect_expr(expr_id, body, infer))
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

    /// Collects `TypeInfo` from types in the signature of a function if it's exposed externally.
    pub fn maybe_collect_fn_signature(&mut self, hir_fn: mun_hir::Function) {
        // If a function is externally visible or contained in the dispatch table, record the types
        // of the signature
        if self.module_group.should_export_fn(self.db, hir_fn)
            || self.dispatch_table.contains(hir_fn)
        {
            self.collect_fn_signature(hir_fn);
        }
    }

    /// Collects unique `TypeInfo` from the specified function signature and body.
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
        for field in fields.into_iter() {
            self.collect_type(self.hir_types.type_id(&field.ty(self.db)));
        }
    }

    /// Constructs a `TypeTable` from all *used* types.
    pub fn build(self) -> TypeTable<'ink> {
        let mut entries = Vec::from_iter(self.entries);
        entries.sort_by(|a, b| a.name.cmp(&b.name));

        let type_info_to_index = entries
            .iter()
            .enumerate()
            .map(|(idx, type_info)| (type_info.clone(), idx))
            .collect();

        // Construct a list of all `ir::TypeInfo`s
        let type_info_ptrs: Value<'ink, [*const std::ffi::c_void]> = entries
            .iter()
            .map(|_| Value::null(self.value_context))
            .into_value(self.value_context);

        // If there are types, introduce a special global that contains all the TypeInfos
        if !type_info_ptrs.is_empty() {
            let _: Global<'ink, [*const std::ffi::c_void]> = type_info_ptrs.into_global(
                TypeTable::NAME,
                self.value_context,
                false,
                Linkage::External,
                None,
            );
        };

        TypeTable {
            entries,
            type_id_to_index: type_info_to_index,
            table_type: type_info_ptrs.get_type(),
        }
    }
}

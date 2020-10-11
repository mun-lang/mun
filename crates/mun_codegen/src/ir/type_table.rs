use super::types as ir;
use crate::{
    ir::dispatch_table::{DispatchTable, FunctionPrototype},
    ir::ty::HirTypeCache,
    type_info::{TypeInfo, TypeInfoData},
    value::{AsValue, CanInternalize, Global, IrValueContext, IterAsIrValue, Value},
    ModuleGroup,
};
use hir::{Body, ExprId, HirDatabase, InferenceResult};
use inkwell::{
    context::Context, module::Linkage, module::Module, targets::TargetData, types::ArrayType,
    values::PointerValue,
};
use std::{
    collections::{BTreeSet, HashMap},
    convert::TryInto,
    ffi::CString,
    mem,
    sync::Arc,
};

/// A type table in IR is a list of pointers to unique type information that are used to generate
/// function and struct information.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TypeTable<'ink> {
    type_info_to_index: HashMap<TypeInfo, usize>,
    table_type: ArrayType<'ink>,
}

impl<'ink> TypeTable<'ink> {
    /// The name of the TypeTable's LLVM `GlobalValue`.
    pub(crate) const NAME: &'static str = "global_type_table";

    /// Looks for a global symbol with the name of the TypeTable global in the specified `module`.
    /// Returns the global value if it could be found, `None` otherwise.
    pub fn find_global(module: &Module<'ink>) -> Option<Global<'ink, [*const ir::TypeInfo<'ink>]>> {
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
        type_info: &TypeInfo,
        table_ref: Option<Global<'ink, [*const ir::TypeInfo<'ink>]>>,
    ) -> PointerValue<'ink> {
        let table_ref = table_ref.expect("no type table defined");

        let index: u64 = (*self
            .type_info_to_index
            .get(type_info)
            .expect("unknown type"))
        .try_into()
        .expect("too many types");

        let global_index = context.i64_type().const_zero();
        let array_index = context.i64_type().const_int(index as u64, false);

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

    /// Retrieves the global `TypeInfo` IR value corresponding to `type_info`, if it exists.
    pub fn get(
        module: &Module<'ink>,
        type_info: &TypeInfo,
        context: &IrValueContext<'ink, '_, '_>,
    ) -> Option<Value<'ink, *const ir::TypeInfo<'ink>>> {
        module
            .get_global(&type_info_global_name(type_info))
            .map(|g| Value::<*const ir::TypeInfo>::with_cast(g.as_pointer_value(), context))
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
    target_data: TargetData,
    value_context: &'t IrValueContext<'ink, 't, 't>,
    dispatch_table: &'t DispatchTable<'ink>,
    hir_types: &'t HirTypeCache<'db, 'ink>,
    entries: BTreeSet<TypeInfo>, // Use a `BTreeSet` to guarantee deterministically ordered output
    module_group: &'t ModuleGroup,
}

impl<'db, 'ink, 't> TypeTableBuilder<'db, 'ink, 't> {
    /// Creates a new `TypeTableBuilder`.
    pub(crate) fn new<'f>(
        db: &'db dyn HirDatabase,
        target_data: TargetData,
        value_context: &'t IrValueContext<'ink, '_, '_>,
        intrinsics: impl Iterator<Item = &'f FunctionPrototype>,
        dispatch_table: &'t DispatchTable<'ink>,
        hir_types: &'t HirTypeCache<'db, 'ink>,
        module_group: &'t ModuleGroup,
    ) -> Self {
        let mut builder = Self {
            db,
            target_data,
            value_context,
            dispatch_table,
            hir_types,
            entries: BTreeSet::new(),
            module_group,
        };

        for prototype in intrinsics {
            for arg_type in prototype.arg_types.iter() {
                builder.collect_type(arg_type.clone());
            }
            if let Some(ret_type) = prototype.ret_type.as_ref() {
                builder.collect_type(ret_type.clone());
            }
        }

        builder
    }

    /// Collects unique `TypeInfo` from the given `Ty`.
    fn collect_type(&mut self, type_info: TypeInfo) {
        if let TypeInfoData::Struct(hir_struct) = type_info.data {
            self.collect_struct(hir_struct);
        } else {
            self.entries.insert(type_info);
        }
    }

    /// Collects unique `TypeInfo` from the specified expression and its sub-expressions.
    fn collect_expr(&mut self, expr_id: ExprId, body: &Arc<Body>, infer: &InferenceResult) {
        let expr = &body[expr_id];

        // If this expression is a call, store it in the dispatch table
        if let hir::Expr::Call { callee, .. } = expr {
            match infer[*callee].as_callable_def() {
                Some(hir::CallableDef::Function(hir_fn)) => {
                    self.maybe_collect_fn_signature(hir_fn);
                }
                Some(hir::CallableDef::Struct(_)) => (),
                None => panic!("expected a callable expression"),
            }
        }

        // Recurse further
        expr.walk_child_exprs(|expr_id| self.collect_expr(expr_id, body, infer))
    }

    /// Collects `TypeInfo` from types in the signature of a function
    pub fn collect_fn_signature(&mut self, hir_fn: hir::Function) {
        let fn_sig = hir_fn.ty(self.db).callable_sig(self.db).unwrap();

        // Collect argument types
        for ty in fn_sig.params().iter() {
            self.collect_type(self.hir_types.type_info(ty));
        }

        // Collect return type
        let ret_ty = fn_sig.ret();
        if !ret_ty.is_empty() {
            self.collect_type(self.hir_types.type_info(ret_ty));
        }
    }

    /// Collects `TypeInfo` from types in the signature of a function if it's exposed externally.
    pub fn maybe_collect_fn_signature(&mut self, hir_fn: hir::Function) {
        // If a function is externally visible or contained in the dispatch table, record the types
        // of the signature
        if self.module_group.should_export_fn(self.db, hir_fn)
            || self.dispatch_table.contains(hir_fn)
        {
            self.collect_fn_signature(hir_fn);
        }
    }

    /// Collects unique `TypeInfo` from the specified function signature and body.
    pub fn collect_fn(&mut self, hir_fn: hir::Function) {
        self.maybe_collect_fn_signature(hir_fn);

        // Collect used types from body
        let body = hir_fn.body(self.db);
        let infer = hir_fn.infer(self.db);
        self.collect_expr(body.body_expr(), &body, &infer);
    }

    /// Collects unique `TypeInfo` from the specified struct type.
    pub fn collect_struct(&mut self, hir_struct: hir::Struct) {
        let type_info = self.hir_types.type_info(&hir_struct.ty(self.db));
        self.entries.insert(type_info);

        let fields = hir_struct.fields(self.db);
        for field in fields.into_iter() {
            self.collect_type(self.hir_types.type_info(&field.ty(self.db)));
        }
    }

    fn gen_type_info(
        &self,
        type_info_to_ir: &mut HashMap<TypeInfo, Value<'ink, *const ir::TypeInfo<'ink>>>,
        type_info: &TypeInfo,
    ) -> Value<'ink, *const ir::TypeInfo<'ink>> {
        // If there is already an entry, return that.
        if let Some(value) = type_info_to_ir.get(type_info) {
            return *value;
        }

        // Construct the header part of the abi::TypeInfo
        let type_info_ir = ir::TypeInfo {
            guid: type_info.guid,
            name: CString::new(type_info.name.clone())
                .expect("typename is not a valid CString")
                .intern(
                    format!("type_info::<{}>::name", type_info.name),
                    self.value_context,
                )
                .as_value(self.value_context),
            size_in_bits: type_info
                .size
                .bit_size
                .try_into()
                .expect("could not convert size in bits to smaller size"),
            alignment: type_info
                .size
                .alignment
                .try_into()
                .expect("could not convert alignment to smaller size"),
            data: self.gen_type_info_data(type_info_to_ir, &type_info.data),
        }
        .as_value(self.value_context);

        // Build the global value for the ir::TypeInfo
        let type_ir_name = type_info_global_name(type_info);
        let value = type_info_ir
            .into_const_private_global(&type_ir_name, self.value_context)
            .as_value(self.value_context);

        // Insert the value in this case, so we don't recompute and generate multiple values.
        type_info_to_ir.insert(type_info.clone(), value);

        value
    }

    fn gen_type_info_data(
        &self,
        type_info_to_ir: &mut HashMap<TypeInfo, Value<'ink, *const ir::TypeInfo<'ink>>>,
        data: &TypeInfoData,
    ) -> ir::TypeInfoData<'ink> {
        match data {
            TypeInfoData::Primitive => ir::TypeInfoData::Primitive,
            TypeInfoData::Struct(s) => {
                ir::TypeInfoData::Struct(self.gen_struct_info(type_info_to_ir, *s))
            }
        }
    }

    fn gen_struct_info(
        &self,
        type_info_to_ir: &mut HashMap<TypeInfo, Value<'ink, *const ir::TypeInfo<'ink>>>,
        hir_struct: hir::Struct,
    ) -> ir::StructInfo<'ink> {
        let struct_ir = self.hir_types.get_struct_type(hir_struct);
        let name = hir_struct.full_name(self.db);
        let fields = hir_struct.fields(self.db);

        // Construct an array of field names (or null if there are no fields)
        let field_names = fields
            .iter()
            .enumerate()
            .map(|(idx, field)| {
                CString::new(field.name(self.db).to_string())
                    .expect("field name is not a valid CString")
                    .intern(
                        format!("struct_info::<{}>::field_names.{}", name, idx),
                        self.value_context,
                    )
                    .as_value(self.value_context)
            })
            .into_const_private_pointer_or_null(
                format!("struct_info::<{}>::field_names", name),
                self.value_context,
            );

        // Construct an array of field types (or null if there are no fields)
        let field_types = fields
            .iter()
            .map(|field| {
                let field_type_info = self.hir_types.type_info(&field.ty(self.db));
                self.gen_type_info(type_info_to_ir, &field_type_info)
            })
            .into_const_private_pointer_or_null(
                format!("struct_info::<{}>::field_types", name),
                self.value_context,
            );

        // Construct an array of field offsets (or null if there are no fields)
        let field_offsets = fields
            .iter()
            .enumerate()
            .map(|(idx, _)| {
                self.target_data
                    .offset_of_element(&struct_ir, idx as u32)
                    .unwrap() as u16
            })
            .into_const_private_pointer_or_null(
                format!("struct_info::<{}>::field_offsets", name),
                self.value_context,
            );

        ir::StructInfo {
            field_names,
            field_types,
            field_offsets,
            num_fields: fields
                .len()
                .try_into()
                .expect("could not convert num_fields to smaller bit size"),
            memory_kind: hir_struct.data(self.db.upcast()).memory_kind.clone(),
        }
    }

    /// Constructs a `TypeTable` from all *used* types.
    pub fn build(mut self) -> TypeTable<'ink> {
        let mut entries = BTreeSet::new();
        mem::swap(&mut entries, &mut self.entries);

        let mut type_info_to_ir = HashMap::with_capacity(entries.len());
        let mut type_info_to_index = HashMap::with_capacity(entries.len());

        // Construct a list of all `ir::TypeInfo`s
        let type_info_ptrs: Value<'ink, [*const ir::TypeInfo]> = entries
            .into_iter()
            .enumerate()
            .map(|(index, type_info)| {
                let ptr = self
                    .gen_type_info(&mut type_info_to_ir, &type_info)
                    .as_value(self.value_context);
                type_info_to_index.insert(type_info, index);
                ptr
            })
            .into_value(self.value_context);

        // If there are types, introduce a special global that contains all the TypeInfos
        if !type_info_ptrs.is_empty() {
            let _: Global<'ink, [*const ir::TypeInfo]> = type_info_ptrs.into_global(
                TypeTable::NAME,
                self.value_context,
                true,
                Linkage::External,
                None,
            );
        };

        TypeTable {
            type_info_to_index,
            table_type: type_info_ptrs.get_type(),
        }
    }
}

fn type_info_global_name(type_info: &TypeInfo) -> String {
    format!("type_info::<{}>", type_info.name)
}

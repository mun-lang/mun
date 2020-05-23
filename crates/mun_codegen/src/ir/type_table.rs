use super::ir_types as ir;
use crate::ir::dispatch_table::{DispatchTable, FunctionPrototype};
use crate::type_info::{TypeGroup, TypeInfo};
use crate::value::{AsValue, CanInternalize, Global, IrValueContext, IterAsIrValue, Value};
use crate::IrDatabase;
use hir::{Body, ExprId, InferenceResult};
use inkwell::module::Linkage;
use inkwell::{module::Module, targets::TargetData, types::ArrayType, values::PointerValue};
use std::collections::{BTreeSet, HashMap};
use std::ffi::CString;
use std::{mem, sync::Arc};

/// A type table in IR is a list of pointers to unique type information that are used to generate
/// function and struct information.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TypeTable {
    type_info_to_index: HashMap<TypeInfo, usize>,
    table_type: ArrayType,
}

impl TypeTable {
    /// The name of the TypeTable's LLVM `GlobalValue`.
    pub(crate) const NAME: &'static str = "global_type_table";

    /// Looks for a global symbol with the name of the TypeTable global in the specified `module`.
    /// Returns the global value if it could be found, `None` otherwise.
    pub fn find_global(module: &Module) -> Option<Global<[*const ir::TypeInfo]>> {
        module.get_global(Self::NAME).map(Global::from_raw)
    }

    /// Generates a `TypeInfo` lookup through the `TypeTable`, equivalent to something along the
    /// lines of: `type_table[i]`, where `i` is the index of the type and `type_table` is an array
    /// of `TypeInfo` pointers.
    pub fn gen_type_info_lookup(
        &self,
        builder: &inkwell::builder::Builder,
        type_info: &TypeInfo,
        table_ref: Option<Global<[*const ir::TypeInfo]>>,
    ) -> PointerValue {
        let table_ref = table_ref.expect("no type table defined");

        let index = *self
            .type_info_to_index
            .get(type_info)
            .expect("unknown type");

        let ptr_to_type_info_ptr = unsafe {
            builder.build_struct_gep(
                table_ref.into(),
                index as u32,
                &format!("{}_ptr_ptr", type_info.name),
            )
        };
        builder
            .build_load(ptr_to_type_info_ptr, &format!("{}_ptr", type_info.name))
            .into_pointer_value()
    }

    /// Retrieves the global `TypeInfo` IR value corresponding to `type_info`, if it exists.
    pub fn get(module: &Module, type_info: &TypeInfo) -> Option<Global<ir::TypeInfo>> {
        module
            .get_global(&type_info_global_name(type_info))
            .map(Global::from_raw)
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
    pub fn ty(&self) -> ArrayType {
        self.table_type
    }
}

/// Used to build a `TypeTable` from HIR.
pub(crate) struct TypeTableBuilder<'a, 'ctx, 'm, D: IrDatabase> {
    db: &'a D,
    target_data: Arc<TargetData>,
    value_context: &'a IrValueContext<'a, 'ctx, 'm>,
    dispatch_table: &'a DispatchTable,
    entries: BTreeSet<TypeInfo>, // Use a `BTreeSet` to guarantee deterministically ordered output
}

impl<'a, 'ctx, 'm, D: IrDatabase> TypeTableBuilder<'a, 'ctx, 'm, D> {
    /// Creates a new `TypeTableBuilder`.
    pub(crate) fn new<'f>(
        db: &'a D,
        value_context: &'a IrValueContext<'a, 'ctx, 'm>,
        intrinsics: impl Iterator<Item = &'f FunctionPrototype>,
        dispatch_table: &'a DispatchTable,
    ) -> Self {
        let mut builder = Self {
            db,
            target_data: db.target_data(),
            value_context,
            dispatch_table,
            entries: BTreeSet::new(),
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
        if let TypeGroup::StructTypes(hir_struct) = type_info.group {
            self.collect_struct(hir_struct);
        } else {
            self.entries.insert(type_info);
        }
    }

    /// Collects unique `TypeInfo` from the specified expression and its sub-expressions.
    fn collect_expr(&mut self, expr_id: ExprId, body: &Arc<Body>, infer: &InferenceResult) {
        let expr = &body[expr_id];

        // TODO: Collect used external `TypeInfo` for the type dispatch table

        // Recurse further
        expr.walk_child_exprs(|expr_id| self.collect_expr(expr_id, body, infer))
    }

    /// Collects unique `TypeInfo` from the specified function signature and body.
    pub fn collect_fn(&mut self, hir_fn: hir::Function) {
        // Collect type info for exposed function
        if !hir_fn.data(self.db).visibility().is_private() || self.dispatch_table.contains(hir_fn) {
            let fn_sig = hir_fn.ty(self.db).callable_sig(self.db).unwrap();

            // Collect argument types
            for ty in fn_sig.params().iter() {
                self.collect_type(self.db.type_info(ty.clone()));
            }

            // Collect return type
            let ret_ty = fn_sig.ret();
            if !ret_ty.is_empty() {
                self.collect_type(self.db.type_info(ret_ty.clone()));
            }
        }

        // Collect used types from body
        let body = hir_fn.body(self.db);
        let infer = hir_fn.infer(self.db);
        self.collect_expr(body.body_expr(), &body, &infer);
    }

    /// Collects unique `TypeInfo` from the specified struct type.
    pub fn collect_struct(&mut self, hir_struct: hir::Struct) {
        let type_info = self.db.type_info(hir_struct.ty(self.db));
        self.entries.insert(type_info);

        let fields = hir_struct.fields(self.db);
        for field in fields.into_iter() {
            self.collect_type(self.db.type_info(field.ty(self.db)));
        }
    }

    fn gen_type_info(
        &self,
        type_info_to_ir: &mut HashMap<TypeInfo, Global<ir::TypeInfo>>,
        type_info: &TypeInfo,
    ) -> Global<ir::TypeInfo> {
        // If there is already an entry, return that.
        if let Some(global) = type_info_to_ir.get(type_info) {
            return *global;
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
            size_in_bits: type_info.size.bit_size as u32,
            alignment: type_info.size.alignment as u8,
            type_group: type_info.group.to_abi_type(),
        }
        .as_value(self.value_context);

        // Build the global value for the ir::TypeInfo
        let type_ir_name = type_info_global_name(type_info);
        let global = match type_info.group {
            TypeGroup::FundamentalTypes => {
                type_info_ir.into_const_private_global(&type_ir_name, self.value_context)
            }
            TypeGroup::StructTypes(s) => {
                // In case of a struct the `Global<ir::TypeInfo>` is actually a
                // `Global<(ir::TypeInfo, ir::StructInfo)>`. We mask this value which is unsafe
                // but correct from an ABI perspective.
                let struct_info_ir = self.gen_struct_info(type_info_to_ir, s);
                let compound_type_ir = (type_info_ir, struct_info_ir).as_value(self.value_context);
                let compound_global =
                    compound_type_ir.into_const_private_global(&type_ir_name, self.value_context);
                Global::from_raw(compound_global.value)
            }
        };

        // Insert the value in the case so we dont recompute and generate multiple values.
        type_info_to_ir.insert(type_info.clone(), global);

        global
    }

    fn gen_struct_info(
        &self,
        type_info_to_ir: &mut HashMap<TypeInfo, Global<ir::TypeInfo>>,
        hir_struct: hir::Struct,
    ) -> Value<ir::StructInfo> {
        let struct_ir = self.db.struct_ty(hir_struct);
        let name = hir_struct.name(self.db).to_string();
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
                let field_type_info = self.db.type_info(field.ty(self.db));
                self.gen_type_info(type_info_to_ir, &field_type_info)
                    .as_value(self.value_context)
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
            num_fields: fields.len() as u16,
            memory_kind: hir_struct.data(self.db).memory_kind.clone(),
        }
        .as_value(self.value_context)
    }

    /// Constructs a `TypeTable` from all *used* types.
    pub fn build(mut self) -> TypeTable {
        let mut entries = BTreeSet::new();
        mem::swap(&mut entries, &mut self.entries);

        let mut type_info_to_ir = HashMap::with_capacity(entries.len());
        let mut type_info_to_index = HashMap::with_capacity(entries.len());

        // Construct a list of all `ir::TypeInfo`s
        let type_info_ptrs: Value<[*const ir::TypeInfo]> = entries
            .into_iter()
            .enumerate()
            .map(|(index, type_info)| {
                let ptr = self
                    .gen_type_info(&mut type_info_to_ir, &type_info)
                    .as_value(self.value_context);
                type_info_to_index.insert(type_info, index);
                ptr
            })
            .as_value(self.value_context);

        // If there are types, introduce a special global that contains all the TypeInfos
        if !type_info_ptrs.is_empty() {
            let _: Global<[*const ir::TypeInfo]> = type_info_ptrs.into_global(
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

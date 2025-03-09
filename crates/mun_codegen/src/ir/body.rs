use std::{collections::HashMap, sync::Arc};

use inkwell::{
    basic_block::BasicBlock,
    builder::Builder,
    context::Context,
    values::{
        AggregateValueEnum, BasicMetadataValueEnum, BasicValueEnum, CallSiteValue, FloatValue,
        FunctionValue, GlobalValue, IntValue, PointerValue, StructValue,
    },
    AddressSpace, FloatPredicate, IntPredicate,
};
use mun_abi as abi;
use mun_hir::{
    ArithOp, BinaryOp, Body, CmpOp, Expr, ExprId, HirDatabase, HirDisplay, InferenceResult,
    Literal, LogicOp, Name, Ordering, Pat, PatId, Path, ResolveBitness, Resolver, Statement,
    TyKind, UnaryOp, ValueNs,
};

use crate::{
    intrinsics,
    ir::{
        dispatch_table::DispatchTable, ty::HirTypeCache, type_table::TypeTable, RuntimeArrayValue,
        RuntimeReferenceValue,
    },
    module_group::ModuleGroup,
    value::Global,
};

type BreakSources<'ink> = Vec<Option<(BasicValueEnum<'ink>, BasicBlock<'ink>)>>;

struct LoopInfo<'ink> {
    break_values: BreakSources<'ink>,
    exit_block: BasicBlock<'ink>,
}

#[derive(Clone)]
pub(crate) struct ExternalGlobals<'ink> {
    pub alloc_handle: Option<GlobalValue<'ink>>,
    pub dispatch_table: Option<GlobalValue<'ink>>,
    pub type_table: Option<Global<'ink, [*const std::ffi::c_void]>>,
}

pub(crate) struct BodyIrGenerator<'db, 'ink, 't> {
    context: &'ink Context,
    db: &'db dyn HirDatabase,
    body: Arc<Body>,
    infer: Arc<InferenceResult>,
    builder: Builder<'ink>,
    fn_value: FunctionValue<'ink>,
    pat_to_param: HashMap<PatId, inkwell::values::BasicValueEnum<'ink>>,
    pat_to_local: HashMap<PatId, inkwell::values::PointerValue<'ink>>,
    pat_to_name: HashMap<PatId, String>,
    function_map: &'t HashMap<mun_hir::Function, FunctionValue<'ink>>,
    dispatch_table: &'t DispatchTable,
    type_table: &'t TypeTable<'ink>,
    hir_types: &'t HirTypeCache<'db>,
    active_loop: Option<LoopInfo<'ink>>,
    hir_function: mun_hir::Function,
    external_globals: ExternalGlobals<'ink>,
    module_group: &'t ModuleGroup,
}

impl<'db, 'ink, 't> BodyIrGenerator<'db, 'ink, 't> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        context: &'ink Context,
        db: &'db dyn HirDatabase,
        function: (mun_hir::Function, FunctionValue<'ink>),
        function_map: &'t HashMap<mun_hir::Function, FunctionValue<'ink>>,
        dispatch_table: &'t DispatchTable,
        type_table: &'t TypeTable<'ink>,
        external_globals: ExternalGlobals<'ink>,
        hir_types: &'t HirTypeCache<'db>,
        module_group: &'t ModuleGroup,
    ) -> Self {
        let (hir_function, ir_function) = function;

        // Get the type information from the `mun_hir::Function`
        let body = hir_function.body(db);
        let infer = hir_function.infer(db);

        // Construct a builder for the IR function
        let builder = context.create_builder();
        let body_ir = context.append_basic_block(ir_function, "body");
        builder.position_at_end(body_ir);

        BodyIrGenerator {
            context,
            db,
            body,
            infer,
            builder,
            fn_value: ir_function,
            pat_to_param: HashMap::default(),
            pat_to_local: HashMap::default(),
            pat_to_name: HashMap::default(),
            function_map,
            dispatch_table,
            type_table,
            active_loop: None,
            hir_function,
            external_globals,
            hir_types,
            module_group,
        }
    }

    /// Generates IR for the body of the function.
    pub fn gen_fn_body(&mut self) {
        // Iterate over all parameters and their type and store them so we can reference
        // them later in code.
        for (i, (pat, _ty)) in self.body.params().iter().enumerate() {
            let body = self.body.clone(); // Avoid borrow issues

            match &body[*pat] {
                Pat::Bind { name } => {
                    let name = name.to_string();
                    let param = self.fn_value.get_nth_param(i as u32).unwrap();
                    let builder = self.new_alloca_builder();
                    let param_ptr = builder.build_alloca(param.get_type(), &name);
                    builder.build_store(param_ptr, param);
                    self.pat_to_local.insert(*pat, param_ptr);
                    self.pat_to_name.insert(*pat, name);
                }
                Pat::Wild => {
                    // Wildcard patterns cannot be referenced from code. So
                    // nothing to do.
                }
                Pat::Path(_) => unreachable!(
                    "Path patterns are not supported as parameters, are we missing a diagnostic?"
                ),
                Pat::Missing => unreachable!(
                    "found missing Pattern, should not be generating IR for incomplete code"
                ),
            }
        }

        // Generate code for the body of the function
        let ret_value = self.gen_expr(self.body.body_expr());

        // Construct a return statement from the returned value of the body if a return
        // is expected in the first place. If the return type of the body is
        // `never` there is no need to generate a return statement.
        let block_ret_type = &self.infer[self.body.body_expr()];
        let fn_ret_type = self
            .hir_function
            .ty(self.db)
            .callable_sig(self.db)
            .unwrap()
            .ret()
            .clone();
        if !block_ret_type.is_never() {
            if fn_ret_type.is_empty() {
                self.builder.build_return(None);
            } else if let Some(value) = ret_value {
                self.builder.build_return(Some(&value));
            }
        }
    }

    pub fn gen_fn_wrapper(&mut self) {
        let fn_sig = self.hir_function.ty(self.db).callable_sig(self.db).unwrap();
        let args: Vec<BasicMetadataValueEnum<'_>> = fn_sig
            .params()
            .iter()
            .enumerate()
            .map(|(idx, ty)| {
                let param = self.fn_value.get_nth_param(idx as u32).unwrap();
                if let Some(s) = ty.as_struct() {
                    if s.data(self.db.upcast()).memory_kind == abi::StructMemoryKind::Value {
                        deref_heap_value(&self.builder, param)
                    } else {
                        param
                    }
                } else {
                    param
                }
                .into()
            })
            .collect();

        let ret_value = self
            .gen_call(self.hir_function, &args)
            .try_as_basic_value()
            .left();

        let call_return_type = &self.infer[self.body.body_expr()];
        if !call_return_type.is_never() {
            let fn_ret_type = self
                .hir_function
                .ty(self.db)
                .callable_sig(self.db)
                .unwrap()
                .ret()
                .clone();

            if fn_ret_type.is_empty() {
                self.builder.build_return(None);
            } else if let Some(value) = ret_value {
                let ret_value = if let Some(hir_struct) = fn_ret_type.as_struct() {
                    if hir_struct.data(self.db.upcast()).memory_kind
                        == mun_hir::StructMemoryKind::Value
                    {
                        self.gen_struct_alloc_on_heap(hir_struct, value.into_struct_value())
                    } else {
                        value
                    }
                } else {
                    value
                };
                self.builder.build_return(Some(&ret_value));
            }
        }
    }

    /// Generates IR for the specified expression. Dependending on the type of
    /// expression an IR value is returned.
    fn gen_expr(&mut self, expr: ExprId) -> Option<inkwell::values::BasicValueEnum<'ink>> {
        let body = self.body.clone();
        match &body[expr] {
            Expr::Block {
                ref statements,
                tail,
            } => self.gen_block(expr, statements, *tail),
            Expr::Path(ref p) => {
                let resolver =
                    mun_hir::resolver_for_expr(self.db.upcast(), self.body.owner(), expr);
                Some(self.gen_path_expr(p, expr, &resolver))
            }
            Expr::Literal(lit) => Some(self.gen_literal(lit, expr)),
            Expr::RecordLit { fields, .. } => Some(self.gen_record_lit(expr, fields)),
            Expr::BinaryOp { lhs, rhs, op } => {
                self.gen_binary_op(expr, *lhs, *rhs, op.expect("missing op"))
            }
            Expr::UnaryOp { expr, op } => self.gen_unary_op(*expr, *op),
            Expr::MethodCall { .. } => {
                unimplemented!("Method calls are not yet implemented in the IR generator")
            }
            Expr::Call {
                ref callee,
                ref args,
            } => {
                // Get the callable definition from the map
                match self.infer[*callee].as_callable_def() {
                    Some(mun_hir::CallableDef::Function(def)) => {
                        // Get all the arguments
                        let args: Vec<BasicMetadataValueEnum<'_>> = args
                            .iter()
                            .map(|expr| self.gen_expr(*expr).expect("expected a value").into())
                            .collect();

                        self.gen_call(def, &args)
                            .try_as_basic_value()
                            .left()
                            // If the called function is a void function it doesn't return anything.
                            // If this method (`gen_expr`) returns None we assume the return value
                            // is `never`. We return a const unit struct here to ensure that at
                            // least something is returned. This matches with the mun_hir where a
                            // `nothing` is returned instead of a `never`.
                            //
                            // This unit value will also be optimized out.
                            .or_else(|| match self.infer[expr].interned() {
                                TyKind::Never => None,
                                _ => Some(self.context.const_struct(&[], false).into()),
                            })
                    }
                    Some(mun_hir::CallableDef::Struct(_)) => {
                        Some(self.gen_named_tuple_lit(expr, args))
                    }
                    None => panic!("expected a callable expression"),
                }
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => self.gen_if(expr, *condition, *then_branch, *else_branch),
            Expr::Return { expr: ret_expr } => self.gen_return(expr, *ret_expr),
            Expr::Loop { body } => self.gen_loop(expr, *body),
            Expr::While { condition, body } => self.gen_while(expr, *condition, *body),
            Expr::Break { expr: break_expr } => self.gen_break(expr, *break_expr),
            Expr::Field {
                expr: receiver_expr,
                name,
            } => self.gen_field(expr, *receiver_expr, name),
            Expr::Array(exprs) => self.gen_array(expr, exprs).map(Into::into),
            Expr::Index { base, index } => self.gen_index(expr, *base, *index),
            Expr::Missing => unimplemented!("unimplemented expr type {:?}", &body[expr]),
        }
    }

    /// Generates an IR value that represents the given `Literal`.
    fn gen_literal(&mut self, lit: &Literal, expr: ExprId) -> BasicValueEnum<'ink> {
        match lit {
            Literal::Int(v) => {
                let ty = match &self.infer[expr].interned() {
                    TyKind::Int(int_ty) => int_ty,
                    _ => unreachable!(
                        "cannot construct an IR value for anything but an integral type"
                    ),
                };

                let context = self.context;
                let ir_ty = match ty.resolve(&self.db.target_data_layout()).bitness {
                    mun_hir::IntBitness::X8 => context.i8_type().const_int(v.value as u64, false),
                    mun_hir::IntBitness::X16 => context.i16_type().const_int(v.value as u64, false),
                    mun_hir::IntBitness::X32 => context.i32_type().const_int(v.value as u64, false),
                    mun_hir::IntBitness::X64 => context.i64_type().const_int(v.value as u64, false),
                    mun_hir::IntBitness::X128 => {
                        context.i128_type().const_int_arbitrary_precision(&unsafe {
                            std::mem::transmute::<u128, [u64; 2]>(v.value)
                        })
                    }
                    mun_hir::IntBitness::Xsize => {
                        unreachable!("unresolved bitness in code generation")
                    }
                };

                ir_ty.into()
            }

            Literal::Float(v) => {
                let ty = &self.infer[expr];
                let ty = match ty.interned()  {
                    TyKind::Float(float_ty) => float_ty,
                    _ => unreachable!("cannot construct an IR value for anything but a float type (literal type: {})", ty.display(self.db)),
                };

                let context = self.context;
                let ir_ty = match ty.bitness.resolve(&self.db.target_data_layout()) {
                    mun_hir::FloatBitness::X32 => context.f32_type().const_float(v.value),
                    mun_hir::FloatBitness::X64 => context.f64_type().const_float(v.value),
                };

                ir_ty.into()
            }

            Literal::Bool(value) => {
                let ty = self.context.bool_type();
                if *value {
                    ty.const_all_ones().into()
                } else {
                    ty.const_zero().into()
                }
            }

            Literal::String(_) => unimplemented!("string literals are not implemented yet"),
        }
    }

    /// Constructs an empty struct value e.g. `{}`
    fn gen_empty(&mut self) -> BasicValueEnum<'ink> {
        self.context.const_struct(&[], false).into()
    }

    /// Allocate a struct literal either on the stack or the heap based on the
    /// type of the struct.
    fn gen_struct_alloc(
        &mut self,
        hir_struct: mun_hir::Struct,
        args: Vec<BasicValueEnum<'ink>>,
    ) -> BasicValueEnum<'ink> {
        // Construct the struct literal
        let struct_ty = self.hir_types.get_struct_type(hir_struct);
        let mut value: AggregateValueEnum<'_> = struct_ty.get_undef().into();
        for (i, arg) in args.into_iter().enumerate() {
            value = self
                .builder
                .build_insert_value(value, arg, i as u32, "init")
                .expect("Failed to initialize struct field.");
        }
        let struct_lit = value.into_struct_value();

        match hir_struct.data(self.db.upcast()).memory_kind {
            mun_hir::StructMemoryKind::Value => struct_lit.into(),
            mun_hir::StructMemoryKind::Gc => {
                // TODO: Root memory in GC
                self.gen_struct_alloc_on_heap(hir_struct, struct_lit)
            }
        }
    }

    fn gen_struct_alloc_on_heap(
        &mut self,
        hir_struct: mun_hir::Struct,
        struct_lit: StructValue<'_>,
    ) -> BasicValueEnum<'ink> {
        let struct_ir_ty = self.hir_types.get_struct_type(hir_struct);
        let new_fn_ptr = self.dispatch_table.gen_intrinsic_lookup(
            self.external_globals.dispatch_table,
            &self.builder,
            &intrinsics::new,
        );

        let type_info_ptr = self.type_table.gen_type_info_lookup(
            self.context,
            &self.builder,
            &self.hir_types.type_id(&hir_struct.ty(self.db)),
            self.external_globals.type_table,
        );

        // HACK: We should be able to use pointers for built-in struct types like
        // `TypeInfo` in intrinsics
        let type_info_ptr = self.builder.build_bitcast(
            type_info_ptr,
            self.context.i8_type().ptr_type(AddressSpace::default()),
            "type_info_ptr_to_i8_ptr",
        );

        let allocator_handle = self.get_allocator_handle_ptr();

        // Safety: we can be sure that the new intrinsic returns a reference.
        let untyped_reference = self
            .builder
            .build_call(
                new_fn_ptr,
                &[type_info_ptr.into(), allocator_handle.into()],
                "ref",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        // Cast the object pointer to the struct type
        let typed_reference = self
            .builder
            .build_bitcast(
                untyped_reference,
                struct_ir_ty
                    .ptr_type(AddressSpace::default())
                    .ptr_type(AddressSpace::default()),
                &format!("ref<{}>", hir_struct.name(self.db)),
            )
            .into_pointer_value();

        // Construct a reference of the object
        let reference = RuntimeReferenceValue::from_ptr(typed_reference, struct_ir_ty)
            .expect("unable to construct mun reference type");

        // Store the struct value
        let struct_ptr = reference.get_data_ptr(&self.builder);
        self.builder.build_store(struct_ptr, struct_lit);

        reference.into()
    }

    /// Generates IR for a record literal, e.g. `Foo { a: 1.23, b: 4 }`
    fn gen_record_lit(
        &mut self,
        type_expr: ExprId,
        fields: &[mun_hir::RecordLitField],
    ) -> BasicValueEnum<'ink> {
        let struct_ty = self.infer[type_expr].clone();
        let hir_struct = struct_ty.as_struct().unwrap(); // Can only really get here if the type is a struct
        let fields: Vec<BasicValueEnum<'ink>> = fields
            .iter()
            .map(|field| self.gen_expr(field.expr).expect("expected a field value"))
            .collect();

        self.gen_struct_alloc(hir_struct, fields)
    }

    /// Generates IR for a named tuple literal, e.g. `Foo(1.23, 4)`
    fn gen_named_tuple_lit(&mut self, type_expr: ExprId, args: &[ExprId]) -> BasicValueEnum<'ink> {
        let struct_ty = self.infer[type_expr].clone();
        let hir_struct = struct_ty.as_struct().unwrap(); // Can only really get here if the type is a struct
        let args: Vec<BasicValueEnum<'ink>> = args
            .iter()
            .map(|expr| self.gen_expr(*expr).expect("expected a field value"))
            .collect();

        self.gen_struct_alloc(hir_struct, args)
    }

    /// Generates IR for a unit struct literal, e.g `Foo`
    fn gen_unit_struct_lit(&mut self, type_expr: ExprId) -> BasicValueEnum<'ink> {
        let struct_ty = self.infer[type_expr].clone();
        let hir_struct = struct_ty.as_struct().unwrap(); // Can only really get here if the type is a struct
        self.gen_struct_alloc(hir_struct, Vec::new())
    }

    /// Generates IR for the specified block expression.
    fn gen_block(
        &mut self,
        _tgt_expr: ExprId,
        statements: &[Statement],
        tail: Option<ExprId>,
    ) -> Option<BasicValueEnum<'ink>> {
        for statement in statements.iter() {
            match statement {
                Statement::Let {
                    pat, initializer, ..
                } => {
                    // If the let statement never finishes, there is no need to generate more code
                    if !self.gen_let_statement(*pat, *initializer) {
                        return None;
                    }
                }
                Statement::Expr(expr) => {
                    // No need to generate code after a statement that has a `never` return type.
                    self.gen_expr(*expr)?;
                }
            };
        }

        if let Some(tail) = tail {
            self.gen_expr(tail)
        } else {
            Some(self.gen_empty())
        }
    }

    /// Constructs a builder that should be used to emit an `alloca`
    /// instruction. These instructions should be at the start of the IR.
    fn new_alloca_builder(&self) -> Builder<'ink> {
        let temp_builder = self.context.create_builder();
        let block = self
            .fn_value
            .get_first_basic_block()
            .expect("at this stage there must be a block");
        if let Some(first_instruction) = block.get_first_instruction() {
            temp_builder.position_before(&first_instruction);
        } else {
            temp_builder.position_at_end(block);
        }
        temp_builder
    }

    /// Generate IR for a let statement: `let a:int = 3`. Returns `false` if the
    /// initializer of the statement never returns; `true` otherwise.
    fn gen_let_statement(&mut self, pat: PatId, initializer: Option<ExprId>) -> bool {
        let initializer = match initializer {
            Some(expr) => match self.gen_expr(expr) {
                Some(expr) => Some(expr),
                None => {
                    // If the initializer doesnt return a value it never returns
                    return false;
                }
            },
            None => None,
        };

        match &self.body[pat] {
            Pat::Bind { name } => {
                let builder = self.new_alloca_builder();
                let pat_ty = self.infer[pat].clone();
                let ty = self
                    .hir_types
                    .get_basic_type(&pat_ty)
                    .expect("expected basic type");
                let ptr = builder.build_alloca(ty, &name.to_string());
                self.pat_to_local.insert(pat, ptr);
                self.pat_to_name.insert(pat, name.to_string());
                if !(pat_ty.is_empty() || pat_ty.is_never()) {
                    if let Some(value) = initializer {
                        self.builder.build_store(ptr, value);
                    };
                }
            }
            Pat::Wild => {}
            Pat::Missing | Pat::Path(_) => unreachable!(),
        }
        true
    }

    /// Generates IR for looking up a certain path expression.
    fn gen_path_expr(
        &mut self,
        path: &Path,
        expr: ExprId,
        resolver: &Resolver,
    ) -> inkwell::values::BasicValueEnum<'ink> {
        match resolver
            .resolve_path_as_value_fully(self.db.upcast(), path)
            .expect("unknown path")
            .0
        {
            ValueNs::ImplSelf(_) => unimplemented!("no support for self types"),
            ValueNs::LocalBinding(pat) => {
                if let Some(param) = self.pat_to_param.get(&pat) {
                    *param
                } else if let Some(ptr) = self.pat_to_local.get(&pat) {
                    let name = self.pat_to_name.get(&pat).expect("could not find pat name");
                    self.builder.build_load(*ptr, name)
                } else {
                    unreachable!("could not find the pattern..");
                }
            }
            ValueNs::StructId(_) => self.gen_unit_struct_lit(expr),
            ValueNs::FunctionId(_) => panic!("unable to generate path expression from a function"),
        }
    }

    /// Given an expression and its value optionally dereference the value to
    /// get to the actual value. This is useful if we need to do an
    /// indirection to get to the actual value.
    fn opt_deref_value(
        &mut self,
        expr: ExprId,
        value: BasicValueEnum<'ink>,
    ) -> BasicValueEnum<'ink> {
        let ty = &self.infer[expr];
        if let Some(s) = ty.as_struct() {
            if s.data(self.db.upcast()).memory_kind == mun_hir::StructMemoryKind::Gc {
                return deref_heap_value(&self.builder, value);
            }
        }
        value
    }

    /// Generates IR for looking up a certain path expression.
    fn gen_path_place_expr(
        &self,
        path: &Path,
        _expr: ExprId,
        resolver: &Resolver,
    ) -> inkwell::values::PointerValue<'ink> {
        match resolver
            .resolve_path_as_value_fully(self.db.upcast(), path)
            .expect("unknown path")
            .0
        {
            ValueNs::ImplSelf(_) => unimplemented!("no support for self types"),
            ValueNs::LocalBinding(pat) => *self
                .pat_to_local
                .get(&pat)
                .expect("unresolved local binding"),
            ValueNs::FunctionId(_) | ValueNs::StructId(_) => {
                panic!("no support for module definitions")
            }
        }
    }

    /// Generates IR to calculate a binary operation between two expressions.
    fn gen_binary_op(
        &mut self,
        _tgt_expr: ExprId,
        lhs: ExprId,
        rhs: ExprId,
        op: BinaryOp,
    ) -> Option<BasicValueEnum<'ink>> {
        let lhs_type = self.infer[lhs].clone();
        match lhs_type.interned() {
            TyKind::Bool => self.gen_binary_op_bool(lhs, rhs, op),
            TyKind::Float(_) => self.gen_binary_op_float(lhs, rhs, op),
            TyKind::Int(ty) => self.gen_binary_op_int(lhs, rhs, op, ty.signedness),
            TyKind::Struct(s) => {
                if s.data(self.db.upcast()).memory_kind == mun_hir::StructMemoryKind::Value {
                    self.gen_binary_op_value_struct(lhs, rhs, op)
                } else {
                    self.gen_binary_op_heap_struct(lhs, rhs, op)
                }
            }
            _ => {
                let rhs_type = self.infer[rhs].clone();
                unimplemented!(
                    "unimplemented operation {0}op{1}",
                    lhs_type.display(self.db),
                    rhs_type.display(self.db)
                )
            }
        }
    }

    /// Generates IR to calculate a unary operation on an expression.
    fn gen_unary_op(&mut self, expr: ExprId, op: UnaryOp) -> Option<BasicValueEnum<'ink>> {
        let ty = &self.infer[expr];
        match ty.interned() {
            TyKind::Float(_) => self.gen_unary_op_float(expr, op),
            &TyKind::Int(int_ty) => self.gen_unary_op_int(expr, op, int_ty.signedness),
            TyKind::Bool => self.gen_unary_op_bool(expr, op),
            _ => unimplemented!("unimplemented operation op{0}", ty.display(self.db)),
        }
    }

    /// Generates IR to calculate a unary operation on a floating point value.
    fn gen_unary_op_float(&mut self, expr: ExprId, op: UnaryOp) -> Option<BasicValueEnum<'ink>> {
        let value: FloatValue<'ink> = self
            .gen_expr(expr)
            .map(|value| self.opt_deref_value(expr, value))
            .expect("no value")
            .into_float_value();
        match op {
            UnaryOp::Neg => Some(self.builder.build_float_neg(value, "neg").into()),
            UnaryOp::Not => unimplemented!("Operator {:?} is not implemented for float", op),
        }
    }

    /// Generates IR to calculate a unary operation on an integer value.
    fn gen_unary_op_int(
        &mut self,
        expr: ExprId,
        op: UnaryOp,
        signedness: mun_hir::Signedness,
    ) -> Option<BasicValueEnum<'ink>> {
        let value: IntValue<'ink> = self
            .gen_expr(expr)
            .map(|value| self.opt_deref_value(expr, value))
            .expect("no value")
            .into_int_value();
        match op {
            UnaryOp::Neg => {
                if signedness == mun_hir::Signedness::Signed {
                    Some(self.builder.build_int_neg(value, "neg").into())
                } else {
                    unimplemented!("Operator {:?} is not implemented for unsigned integer", op)
                }
            }
            UnaryOp::Not => Some(self.builder.build_not(value, "not").into()),
            //_ => unimplemented!("Operator {:?} is not implemented for integer", op),
        }
    }

    /// Generates IR to calculate a unary operation on a boolean value.
    fn gen_unary_op_bool(&mut self, expr: ExprId, op: UnaryOp) -> Option<BasicValueEnum<'ink>> {
        let value: IntValue<'ink> = self
            .gen_expr(expr)
            .map(|value| self.opt_deref_value(expr, value))
            .expect("no value")
            .into_int_value();
        match op {
            UnaryOp::Not => Some(self.builder.build_not(value, "not").into()),
            UnaryOp::Neg => unimplemented!("Operator {:?} is not implemented for boolean", op),
        }
    }

    /// Generates IR to calculate a binary operation between two boolean value.
    fn gen_binary_op_bool(
        &mut self,
        lhs_expr: ExprId,
        rhs_expr: ExprId,
        op: BinaryOp,
    ) -> Option<BasicValueEnum<'ink>> {
        let lhs: IntValue<'ink> = self
            .gen_expr(lhs_expr)
            .map(|value| self.opt_deref_value(lhs_expr, value))?
            .into_int_value();
        let rhs: IntValue<'ink> = self
            .gen_expr(rhs_expr)
            .map(|value| self.opt_deref_value(rhs_expr, value))?
            .into_int_value();
        match op {
            BinaryOp::ArithOp(op) => Some(self.gen_arith_bin_op_bool(lhs, rhs, op).into()),
            BinaryOp::Assignment { op } => {
                let rhs = match op {
                    Some(op) => self.gen_arith_bin_op_bool(lhs, rhs, op),
                    None => rhs,
                };
                let place = self.gen_place_expr(lhs_expr)?;
                self.builder.build_store(place, rhs);
                Some(self.gen_empty())
            }
            BinaryOp::LogicOp(op) => Some(self.gen_logic_bin_op(lhs, rhs, op).into()),
            BinaryOp::CmpOp(op) => Some(
                self.gen_cmp_bin_op_int(lhs, rhs, op, mun_hir::Signedness::Unsigned)
                    .into(),
            ),
        }
    }

    /// Generates IR to calculate a binary operation between two floating point
    /// values.
    fn gen_binary_op_float(
        &mut self,
        lhs_expr: ExprId,
        rhs_expr: ExprId,
        op: BinaryOp,
    ) -> Option<BasicValueEnum<'ink>> {
        let lhs = self
            .gen_expr(lhs_expr)
            .map(|value| self.opt_deref_value(lhs_expr, value))
            .expect("no lhs value")
            .into_float_value();
        let rhs = self
            .gen_expr(rhs_expr)
            .map(|value| self.opt_deref_value(rhs_expr, value))
            .expect("no rhs value")
            .into_float_value();
        match op {
            BinaryOp::ArithOp(op) => Some(self.gen_arith_bin_op_float(lhs, rhs, op).into()),
            BinaryOp::CmpOp(op) => {
                let (name, predicate) = match op {
                    CmpOp::Eq { negated: false } => ("eq", FloatPredicate::OEQ),
                    CmpOp::Eq { negated: true } => ("neq", FloatPredicate::ONE),
                    CmpOp::Ord {
                        ordering: Ordering::Less,
                        strict: false,
                    } => ("lesseq", FloatPredicate::OLE),
                    CmpOp::Ord {
                        ordering: Ordering::Less,
                        strict: true,
                    } => ("less", FloatPredicate::OLT),
                    CmpOp::Ord {
                        ordering: Ordering::Greater,
                        strict: false,
                    } => ("greatereq", FloatPredicate::OGE),
                    CmpOp::Ord {
                        ordering: Ordering::Greater,
                        strict: true,
                    } => ("greater", FloatPredicate::OGT),
                };
                Some(
                    self.builder
                        .build_float_compare(predicate, lhs, rhs, name)
                        .into(),
                )
            }
            BinaryOp::Assignment { op } => {
                let rhs = match op {
                    Some(op) => self.gen_arith_bin_op_float(lhs, rhs, op),
                    None => rhs,
                };
                let place = self.gen_place_expr(lhs_expr)?;
                self.builder.build_store(place, rhs);
                Some(self.gen_empty())
            }
            BinaryOp::LogicOp(_) => {
                unimplemented!("Operator {:?} is not implemented for float", op)
            }
        }
    }

    /// Generates IR to calculate a binary operation between two integer values.
    fn gen_binary_op_int(
        &mut self,
        lhs_expr: ExprId,
        rhs_expr: ExprId,
        op: BinaryOp,
        signedness: mun_hir::Signedness,
    ) -> Option<BasicValueEnum<'ink>> {
        let lhs = self
            .gen_expr(lhs_expr)
            .map(|value| self.opt_deref_value(lhs_expr, value))
            .expect("no lhs value")
            .into_int_value();
        let rhs = self
            .gen_expr(rhs_expr)
            .map(|value| self.opt_deref_value(rhs_expr, value))
            .expect("no rhs value")
            .into_int_value();
        match op {
            BinaryOp::ArithOp(op) => {
                Some(self.gen_arith_bin_op_int(lhs, rhs, op, signedness).into())
            }
            BinaryOp::CmpOp(op) => Some(self.gen_cmp_bin_op_int(lhs, rhs, op, signedness).into()),
            BinaryOp::Assignment { op } => {
                let rhs = match op {
                    Some(op) => self.gen_arith_bin_op_int(lhs, rhs, op, signedness),
                    None => rhs,
                };
                let place = self.gen_place_expr(lhs_expr)?;
                self.builder.build_store(place, rhs);
                Some(self.gen_empty())
            }
            BinaryOp::LogicOp(_) => {
                unreachable!("Operator {:?} is not implemented for integer", op)
            }
        }
    }

    /// Generates IR to calculate a binary operation between two heap struct
    /// values (e.g. a Mun `struct(gc)`).
    fn gen_binary_op_heap_struct(
        &mut self,
        lhs_expr: ExprId,
        rhs_expr: ExprId,
        op: BinaryOp,
    ) -> Option<BasicValueEnum<'ink>> {
        let rhs = self
            .gen_expr(rhs_expr)
            .expect("no rhs value")
            .into_pointer_value();
        match op {
            BinaryOp::Assignment { op } => {
                let rhs = match op {
                    Some(op) => unimplemented!(
                        "Assignment with {:?} operator is not implemented for struct",
                        op
                    ),
                    None => rhs,
                };
                let place = self.gen_place_expr(lhs_expr)?;
                self.builder.build_store(place, rhs);
                Some(self.gen_empty())
            }
            _ => unimplemented!("Operator {:?} is not implemented for struct", op),
        }
    }

    /// Generates IR to calculate a binary operation between two value struct
    /// values, denoted in Mun as `struct(value)`.
    fn gen_binary_op_value_struct(
        &mut self,
        lhs_expr: ExprId,
        rhs_expr: ExprId,
        op: BinaryOp,
    ) -> Option<BasicValueEnum<'ink>> {
        let rhs = self
            .gen_expr(rhs_expr)
            .expect("no rhs value")
            .into_struct_value();
        match op {
            BinaryOp::Assignment { op } => {
                let rhs = match op {
                    Some(op) => unimplemented!(
                        "Assignment with {:?} operator is not implemented for struct",
                        op
                    ),
                    None => rhs,
                };
                let place = self.gen_place_expr(lhs_expr)?;
                self.builder.build_store(place, rhs);
                Some(self.gen_empty())
            }
            _ => unimplemented!("Operator {:?} is not implemented for struct", op),
        }
    }

    fn gen_arith_bin_op_bool(
        &mut self,
        lhs: IntValue<'ink>,
        rhs: IntValue<'ink>,
        op: ArithOp,
    ) -> IntValue<'ink> {
        match op {
            ArithOp::BitAnd => self.builder.build_and(lhs, rhs, "bit_and"),
            ArithOp::BitOr => self.builder.build_or(lhs, rhs, "bit_or"),
            ArithOp::BitXor => self.builder.build_xor(lhs, rhs, "bit_xor"),
            _ => unimplemented!(
                "Assignment with {:?} operator is not implemented for boolean",
                op
            ),
        }
    }

    fn gen_cmp_bin_op_int(
        &mut self,
        lhs: IntValue<'ink>,
        rhs: IntValue<'ink>,
        op: CmpOp,
        signedness: mun_hir::Signedness,
    ) -> IntValue<'ink> {
        let (name, predicate) = match op {
            CmpOp::Eq { negated: false } => ("eq", IntPredicate::EQ),
            CmpOp::Eq { negated: true } => ("neq", IntPredicate::NE),
            CmpOp::Ord {
                ordering: Ordering::Less,
                strict: false,
            } => (
                "lesseq",
                match signedness {
                    mun_hir::Signedness::Signed => IntPredicate::SLE,
                    mun_hir::Signedness::Unsigned => IntPredicate::ULE,
                },
            ),
            CmpOp::Ord {
                ordering: Ordering::Less,
                strict: true,
            } => (
                "less",
                match signedness {
                    mun_hir::Signedness::Signed => IntPredicate::SLT,
                    mun_hir::Signedness::Unsigned => IntPredicate::ULT,
                },
            ),
            CmpOp::Ord {
                ordering: Ordering::Greater,
                strict: false,
            } => (
                "greatereq",
                match signedness {
                    mun_hir::Signedness::Signed => IntPredicate::SGE,
                    mun_hir::Signedness::Unsigned => IntPredicate::UGE,
                },
            ),
            CmpOp::Ord {
                ordering: Ordering::Greater,
                strict: true,
            } => (
                "greater",
                match signedness {
                    mun_hir::Signedness::Signed => IntPredicate::SGT,
                    mun_hir::Signedness::Unsigned => IntPredicate::UGT,
                },
            ),
        };

        self.builder.build_int_compare(predicate, lhs, rhs, name)
    }

    fn gen_arith_bin_op_int(
        &mut self,
        lhs: IntValue<'ink>,
        rhs: IntValue<'ink>,
        op: ArithOp,
        signedness: mun_hir::Signedness,
    ) -> IntValue<'ink> {
        match op {
            ArithOp::Add => self.builder.build_int_add(lhs, rhs, "add"),
            ArithOp::Subtract => self.builder.build_int_sub(lhs, rhs, "sub"),
            ArithOp::Divide => match signedness {
                mun_hir::Signedness::Signed => self.builder.build_int_signed_div(lhs, rhs, "div"),
                mun_hir::Signedness::Unsigned => {
                    self.builder.build_int_unsigned_div(lhs, rhs, "div")
                }
            },
            ArithOp::Multiply => self.builder.build_int_mul(lhs, rhs, "mul"),
            ArithOp::Remainder => match signedness {
                mun_hir::Signedness::Signed => self.builder.build_int_signed_rem(lhs, rhs, "rem"),
                mun_hir::Signedness::Unsigned => {
                    self.builder.build_int_unsigned_rem(lhs, rhs, "rem")
                }
            },
            ArithOp::LeftShift => self.builder.build_left_shift(lhs, rhs, "left_shift"),
            ArithOp::RightShift => {
                self.builder
                    .build_right_shift(lhs, rhs, signedness.is_signed(), "right_shift")
            }
            ArithOp::BitAnd => self.builder.build_and(lhs, rhs, "bit_and"),
            ArithOp::BitOr => self.builder.build_or(lhs, rhs, "bit_or"),
            ArithOp::BitXor => self.builder.build_xor(lhs, rhs, "bit_xor"),
        }
    }

    fn gen_arith_bin_op_float(
        &mut self,
        lhs: FloatValue<'ink>,
        rhs: FloatValue<'ink>,
        op: ArithOp,
    ) -> FloatValue<'ink> {
        match op {
            ArithOp::Add => self.builder.build_float_add(lhs, rhs, "add"),
            ArithOp::Subtract => self.builder.build_float_sub(lhs, rhs, "sub"),
            ArithOp::Divide => self.builder.build_float_div(lhs, rhs, "div"),
            ArithOp::Multiply => self.builder.build_float_mul(lhs, rhs, "mul"),
            ArithOp::Remainder => self.builder.build_float_rem(lhs, rhs, "rem"),
            ArithOp::LeftShift
            | ArithOp::RightShift
            | ArithOp::BitAnd
            | ArithOp::BitOr
            | ArithOp::BitXor => {
                unreachable!("Operator {:?} is not implemented for float", op)
            }
        }
    }

    fn gen_logic_bin_op(
        &mut self,
        lhs: IntValue<'ink>,
        rhs: IntValue<'ink>,
        op: LogicOp,
    ) -> IntValue<'ink> {
        match op {
            LogicOp::And => self.builder.build_and(lhs, rhs, "and"),
            LogicOp::Or => self.builder.build_or(lhs, rhs, "or"),
        }
    }

    /// Given an expression generate code that results in a memory address that
    /// can be used for other place operations.
    fn gen_place_expr(&mut self, expr: ExprId) -> Option<PointerValue<'ink>> {
        let body = self.body.clone();
        match &body[expr] {
            Expr::Path(ref p) => {
                let resolver =
                    mun_hir::resolver_for_expr(self.db.upcast(), self.body.owner(), expr);
                Some(self.gen_path_place_expr(p, expr, &resolver))
            }
            Expr::Field {
                expr: receiver_expr,
                name,
            } => self.gen_place_field(expr, *receiver_expr, name),
            Expr::Index { base, index } => self.gen_place_index(expr, *base, *index),
            _ => unreachable!("invalid place expression"),
        }
    }

    /// Returns true if the specified expression refers to an expression that
    /// results in a memory address that can be used for other place
    /// operations.
    fn is_place_expr(&self, expr: ExprId) -> bool {
        let body = self.body.clone();
        match &body[expr] {
            Expr::Path(..) | Expr::Array(_) => true,
            Expr::Field { expr, .. } => self.is_place_expr(*expr),
            Expr::Index { base, .. } => self.is_place_expr(*base),
            _ => false,
        }
    }

    /// Returns true if a call to the specified function should be looked up in
    /// the dispatch table; if false is returned the function should be
    /// called directly.
    fn should_use_dispatch_table(&self, function: mun_hir::Function) -> bool {
        self.module_group.should_runtime_link_fn(self.db, function)
    }

    /// Generates IR for a function call.
    fn gen_call(
        &mut self,
        function: mun_hir::Function,
        args: &[BasicMetadataValueEnum<'ink>],
    ) -> CallSiteValue<'ink> {
        if self.should_use_dispatch_table(function) {
            let ptr_value = self.dispatch_table.gen_function_lookup(
                self.db,
                self.external_globals.dispatch_table,
                &self.builder,
                function,
            );
            self.builder
                .build_call(ptr_value, args, &function.name(self.db).to_string())
        } else {
            let llvm_function = self.function_map.get(&function).unwrap_or_else(|| {
                panic!(
                    "missing function value for mun_hir function: '{}'",
                    function.name(self.db),
                )
            });
            self.builder
                .build_call(*llvm_function, args, &function.name(self.db).to_string())
        }
    }

    /// Generates IR for an if statement.
    fn gen_if(
        &mut self,
        _expr: ExprId,
        condition: ExprId,
        then_branch: ExprId,
        else_branch: Option<ExprId>,
    ) -> Option<inkwell::values::BasicValueEnum<'ink>> {
        // Generate IR for the condition
        let condition_ir = self
            .gen_expr(condition)
            .map(|value| self.opt_deref_value(condition, value))?
            .into_int_value();

        // Generate the code blocks to branch to
        let mut then_block = self.context.append_basic_block(self.fn_value, "then");
        let else_block_and_expr = match &else_branch {
            Some(else_branch) => Some((
                self.context.append_basic_block(self.fn_value, "else"),
                else_branch,
            )),
            None => None,
        };
        let merge_block = self.context.append_basic_block(self.fn_value, "if_merge");

        // Build the actual branching IR for the if statement
        let else_block = else_block_and_expr.map_or(merge_block, |e| e.0);
        self.builder
            .build_conditional_branch(condition_ir, then_block, else_block);

        // Fill the then block
        self.builder.position_at_end(then_block);
        let then_block_ir = self.gen_expr(then_branch);
        if !self.infer[then_branch].is_never() {
            self.builder.build_unconditional_branch(merge_block);
        }
        then_block = self.builder.get_insert_block().unwrap();

        // Fill the else block, if it exists and get the result back
        let else_ir_and_block = if let Some((else_block, else_branch)) = else_block_and_expr {
            else_block
                .move_after(then_block)
                .expect("programmer error, then_block is invalid");
            self.builder.position_at_end(else_block);
            let result_ir = self.gen_expr(*else_branch);
            if result_ir.is_some() {
                self.builder.build_unconditional_branch(merge_block);
            }
            Some((result_ir, self.builder.get_insert_block().unwrap()))
        } else {
            None
        };

        // Create merge block
        let current_block = self.builder.get_insert_block().unwrap();
        merge_block.move_after(current_block).unwrap();
        self.builder.position_at_end(merge_block);

        // Construct phi block if a value was returned
        if let Some(then_block_ir) = then_block_ir {
            if let Some((Some(else_block_ir), else_block)) = else_ir_and_block {
                let phi = self.builder.build_phi(then_block_ir.get_type(), "iftmp");
                phi.add_incoming(&[(&then_block_ir, then_block), (&else_block_ir, else_block)]);
                Some(phi.as_basic_value())
            } else {
                Some(then_block_ir)
            }
        } else if let Some((else_block_ir, _else_block)) = else_ir_and_block {
            // If both the then and the else block never return, the entire if statement
            // will never return. Therefor we have to remove the merge block
            // because it has no predecessor.
            if else_block_ir.is_none() {
                merge_block
                    .remove_from_function()
                    .expect("merge block must have a parent");
            }
            else_block_ir
        } else {
            Some(self.gen_empty())
        }
    }

    fn gen_return(
        &mut self,
        _expr: ExprId,
        ret_expr: Option<ExprId>,
    ) -> Option<BasicValueEnum<'ink>> {
        let ret_value = ret_expr.and_then(|expr| self.gen_expr(expr));

        // Construct a return statement from the returned value of the body
        if let Some(value) = ret_value {
            self.builder.build_return(Some(&value));
        } else {
            self.builder.build_return(None);
        }

        None
    }

    fn gen_break(
        &mut self,
        _expr: ExprId,
        break_expr: Option<ExprId>,
    ) -> Option<BasicValueEnum<'ink>> {
        if let Some(expr) = break_expr {
            // There is an expression
            // e.g. break x;
            // Turn that expression into IR.
            let break_value = self.gen_expr(expr);

            // If the expression never returns, we can stop what we're doing.
            if let Some(break_value) = break_value {
                let loop_info = self.active_loop.as_mut().unwrap();
                loop_info.break_values.push(Some((
                    break_value,
                    self.builder.get_insert_block().unwrap(),
                )));
                self.builder
                    .build_unconditional_branch(loop_info.exit_block);
            }
        } else {
            // If the break expression doesnt contain a break statement. Add a none to the
            // break values.
            let loop_info = self.active_loop.as_mut().unwrap();
            loop_info.break_values.push(None);
            self.builder
                .build_unconditional_branch(loop_info.exit_block);
        };

        None
    }

    fn gen_loop_block_expr(
        &mut self,
        block: ExprId,
        exit_block: BasicBlock<'ink>,
    ) -> (
        BasicBlock<'ink>,
        BreakSources<'ink>,
        Option<BasicValueEnum<'ink>>,
    ) {
        // Build a new loop info struct
        let loop_info = LoopInfo {
            exit_block,
            break_values: Vec::new(),
        };

        // Replace previous loop info
        let prev_loop = std::mem::replace(&mut self.active_loop, Some(loop_info));

        // Start generating code inside the loop
        let value = self.gen_expr(block);

        let LoopInfo {
            exit_block,
            break_values,
        } = std::mem::replace(&mut self.active_loop, prev_loop).unwrap();

        (exit_block, break_values, value)
    }

    fn gen_while(
        &mut self,
        _expr: ExprId,
        condition_expr: ExprId,
        body_expr: ExprId,
    ) -> Option<BasicValueEnum<'ink>> {
        let context = self.context;
        let cond_block = context.append_basic_block(self.fn_value, "whilecond");
        let loop_block = context.append_basic_block(self.fn_value, "while");
        let exit_block = context.append_basic_block(self.fn_value, "afterwhile");

        // Insert an explicit fall through from the current block to the condition check
        self.builder.build_unconditional_branch(cond_block);

        // Generate condition block
        self.builder.position_at_end(cond_block);
        let condition_ir = self
            .gen_expr(condition_expr)
            .map(|value| self.opt_deref_value(condition_expr, value));
        if let Some(condition_ir) = condition_ir {
            self.builder.build_conditional_branch(
                condition_ir.into_int_value(),
                loop_block,
                exit_block,
            );
        } else {
            // If the condition doesn't return a value, we also immediately return without a
            // value. This can happen if the expression is a `never` expression.
            return None;
        }

        // Generate loop block
        self.builder.position_at_end(loop_block);
        let (exit_block, _, value) = self.gen_loop_block_expr(body_expr, exit_block);
        if value.is_some() {
            self.builder.build_unconditional_branch(cond_block);
        }

        // Generate exit block
        self.builder.position_at_end(exit_block);

        Some(self.gen_empty())
    }

    fn gen_loop(&mut self, _expr: ExprId, body_expr: ExprId) -> Option<BasicValueEnum<'ink>> {
        let context = self.context;
        let loop_block = context.append_basic_block(self.fn_value, "loop");
        let exit_block = context.append_basic_block(self.fn_value, "exit");

        // Insert an explicit fall through from the current block to the loop
        self.builder.build_unconditional_branch(loop_block);

        // Generate the body of the loop
        self.builder.position_at_end(loop_block);
        let (exit_block, break_values, value) = self.gen_loop_block_expr(body_expr, exit_block);
        if value.is_some() {
            self.builder.build_unconditional_branch(loop_block);
        }

        if break_values.is_empty() {
            // Not a single code entry point jumped to the exit block through a break.
            // Therefor we can completely remove the exit block since it doesnt
            // have a predecessor.
            exit_block
                .remove_from_function()
                .expect("the exit block must have a parent");
            None
        } else {
            // Move the builder to the exit block
            self.builder.position_at_end(exit_block);

            // If the break values contain values, (so there where `break x;` statements),
            // generate a phi value. This then assumes that all breaks had
            // values.
            if let Some(Some((value, _))) = break_values.first() {
                let phi = self.builder.build_phi(value.get_type(), "exit");
                for (value, block) in break_values.into_iter().map(Option::unwrap) {
                    phi.add_incoming(&[(&value, block)]);
                }
                Some(phi.as_basic_value())
            } else {
                // Otherwise, in the case of `break;` (without an expression) the return value
                // is just empty.
                Some(self.gen_empty())
            }
        }
    }

    fn gen_field(
        &mut self,
        _expr: ExprId,
        receiver_expr: ExprId,
        name: &Name,
    ) -> Option<BasicValueEnum<'ink>> {
        let hir_struct = self.infer[receiver_expr]
            .as_struct()
            .expect("expected a struct");

        let hir_struct_name = hir_struct.name(self.db);

        let field_idx = hir_struct
            .field(self.db, name)
            .expect("expected a struct field")
            .index(self.db);

        let field_ir_name = &format!("{hir_struct_name}.{name}");
        if self.is_place_expr(receiver_expr) {
            let receiver_ptr = self.gen_place_expr(receiver_expr)?;
            let receiver_ptr = self
                .opt_deref_value(receiver_expr, receiver_ptr.into())
                .into_pointer_value();
            let field_ptr = self
                .builder
                .build_struct_gep(
                    receiver_ptr,
                    field_idx,
                    &format!("{hir_struct_name}->{name}"),
                )
                .unwrap_or_else(|_| {
                    panic!(
                        "could not get pointer to field `{hir_struct_name}::{name}` at index {field_idx}"
                    )
                });
            Some(self.builder.build_load(field_ptr, field_ir_name))
        } else {
            let receiver_value = self.gen_expr(receiver_expr)?;
            let receiver_value = self.opt_deref_value(receiver_expr, receiver_value);
            let receiver_struct = receiver_value.into_struct_value();
            Some(
                self.builder
                    .build_extract_value(receiver_struct, field_idx, field_ir_name)
                    .ok_or_else(|| {
                        format!(
                            "could not extract field {name} (index: {field_idx}) from struct {hir_struct_name}"
                        )
                    })
                    .unwrap(),
            )
        }
    }

    fn gen_place_field(
        &mut self,
        _expr: ExprId,
        receiver_expr: ExprId,
        name: &Name,
    ) -> Option<PointerValue<'ink>> {
        let hir_struct = self.infer[receiver_expr]
            .as_struct()
            .expect("expected a struct");

        let hir_struct_name = hir_struct.name(self.db);

        let field_idx = hir_struct
            .field(self.db, name)
            .expect("expected a struct field")
            .index(self.db);

        let receiver_ptr = self.gen_place_expr(receiver_expr)?;
        let receiver_ptr = self
            .opt_deref_value(receiver_expr, receiver_ptr.into())
            .into_pointer_value();
        Some(
            self.builder
                .build_struct_gep(
                    receiver_ptr,
                    field_idx,
                    &format!("{hir_struct_name}->{name}"),
                )
                .unwrap_or_else(|_| {
                    panic!(
                        "could not get pointer to field `{hir_struct_name}::{name}` at index {field_idx}"
                    )
                }),
        )
    }

    /// Generates code to construct an array literal at runtime. Returns `None`
    /// if the code generation for the array literal never returns.
    fn gen_array(&mut self, expr: ExprId, exprs: &[ExprId]) -> Option<RuntimeArrayValue<'ink>> {
        let array_ty = &self.infer[expr];
        let element_ty = array_ty
            .as_array()
            .expect("the type of an array literal expression must be an Array");

        let new_array_fn_ptr = self.dispatch_table.gen_intrinsic_lookup(
            self.external_globals.dispatch_table,
            &self.builder,
            &intrinsics::new_array,
        );

        let type_info_ptr = self.type_table.gen_type_info_lookup(
            self.context,
            &self.builder,
            &self.hir_types.type_id(array_ty),
            self.external_globals.type_table,
        );

        // HACK: We should be able to use pointers for built-in struct types like
        // `TypeInfo` in intrinsics
        let type_info_ptr = self.builder.build_bitcast(
            type_info_ptr,
            self.context.i8_type().ptr_type(AddressSpace::default()),
            "type_info_ptr_to_i8_ptr",
        );

        let allocator_handle = self.get_allocator_handle_ptr();

        let length_value = self
            .hir_types
            .get_usize_type()
            .const_int(exprs.len() as u64, false);

        // An object pointer adds an extra layer of indirection to allow for hot
        // reloading. To make it struct type agnostic, it is stored in a `*const
        // *mut std::ffi::c_void`.
        let untyped_array_ptr = self
            .builder
            .build_call(
                new_array_fn_ptr,
                &[
                    type_info_ptr.into(),
                    length_value.into(),
                    allocator_handle.into(),
                ],
                "ref",
            )
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        // Cast the object pointer to the array struct type
        let array_ty = self.hir_types.get_array_type(element_ty);
        let array_ptr = self
            .builder
            .build_bitcast(
                untyped_array_ptr,
                array_ty
                    .ptr_type(AddressSpace::default())
                    .ptr_type(AddressSpace::default()),
                &format!("ref<[{}]>", element_ty.display(self.db)),
            )
            .into_pointer_value();

        let array = RuntimeArrayValue::from_ptr(array_ptr, array_ty)
            .expect("unable to convert pointer to typed reference");
        let array_elements = array.get_elements(&self.builder);
        for (idx, expr) in exprs.iter().enumerate() {
            let element_ptr = unsafe {
                self.builder.build_gep(
                    array_elements,
                    &[self.context.i64_type().const_int(idx as u64, false)],
                    &format!("{}[{}]", array_elements.get_name().to_string_lossy(), idx),
                )
            };

            let expr_value = self.gen_expr(*expr)?;
            self.builder.build_store(element_ptr, expr_value);
        }

        // Once all values have been stored in the array, update the length of the array
        let length = array.length_ty().const_int(exprs.len() as u64, false);
        let array_length_ptr = array.get_length_ptr(&self.builder);
        self.builder.build_store(array_length_ptr, length);

        Some(array)
    }

    /// Generates an index into an array
    fn gen_index(
        &mut self,
        expr: ExprId,
        base: ExprId,
        index: ExprId,
    ) -> Option<BasicValueEnum<'ink>> {
        let element_ptr = self.gen_place_index(expr, base, index)?;
        Some(self.builder.build_load(element_ptr, ""))
    }

    /// Generates an index into an array
    fn gen_place_index(
        &mut self,
        _expr: ExprId,
        base: ExprId,
        index: ExprId,
    ) -> Option<PointerValue<'ink>> {
        // Safety: place expression can only be generated if the base expression is an
        // array.
        let base = unsafe {
            RuntimeArrayValue::from_ptr_unchecked(self.gen_expr(base)?.into_pointer_value())
        };
        let index = self.gen_expr(index)?.into_int_value();

        let elements = base.get_elements(&self.builder);
        Some(unsafe {
            self.builder.build_gep(
                elements,
                &[index],
                &format!("{}+index", elements.get_name().to_string_lossy()),
            )
        })
    }

    /// Returns a pointer to the allocator handle
    fn get_allocator_handle_ptr(&self) -> PointerValue<'ink> {
        self.builder
            .build_load(
                self.external_globals
                    .alloc_handle
                    .expect("no allocator handle was specified, this is required for structs")
                    .as_pointer_value(),
                "allocator_handle",
            )
            .into_pointer_value()
    }
}

/// Derefs a heap-allocated value. As we introduce a layer of indirection for
/// hot reloading, we need to first load the pointer that points to the memory
/// block.
fn deref_heap_value<'ink>(
    builder: &Builder<'ink>,
    value: BasicValueEnum<'ink>,
) -> BasicValueEnum<'ink> {
    // Safety: we can assume that the input is a RuntimeReferenceValue
    let mem_ptr = unsafe { RuntimeReferenceValue::from_ptr_unchecked(value.into_pointer_value()) }
        .get_data_ptr(builder);

    builder.build_load(mem_ptr, "deref")
}

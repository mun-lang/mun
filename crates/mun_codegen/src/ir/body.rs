use crate::intrinsics;
use crate::{
    ir::dispatch_table::DispatchTable, ir::try_convert_any_to_basic, CodeGenParams, IrDatabase,
};
use hir::{
    ArenaId, ArithOp, BinaryOp, Body, CmpOp, Expr, ExprId, HirDisplay, InferenceResult, Literal,
    Name, Ordering, Pat, PatId, Path, Resolution, Resolver, Statement, TypeCtor,
};
use inkwell::{
    builder::Builder,
    module::Module,
    values::{BasicValueEnum, CallSiteValue, FloatValue, FunctionValue, IntValue, StructValue},
    AddressSpace, FloatPredicate, IntPredicate,
};
use std::{collections::HashMap, mem, sync::Arc};

use inkwell::basic_block::BasicBlock;
use inkwell::values::{AggregateValueEnum, PointerValue};

struct LoopInfo {
    break_values: Vec<(
        inkwell::values::BasicValueEnum,
        inkwell::basic_block::BasicBlock,
    )>,
    exit_block: BasicBlock,
}

pub(crate) struct BodyIrGenerator<'a, 'b, D: IrDatabase> {
    db: &'a D,
    module: &'a Module,
    body: Arc<Body>,
    infer: Arc<InferenceResult>,
    builder: Builder,
    fn_value: FunctionValue,
    pat_to_param: HashMap<PatId, inkwell::values::BasicValueEnum>,
    pat_to_local: HashMap<PatId, inkwell::values::PointerValue>,
    pat_to_name: HashMap<PatId, String>,
    function_map: &'a HashMap<hir::Function, FunctionValue>,
    dispatch_table: &'b DispatchTable,
    active_loop: Option<LoopInfo>,
    hir_function: hir::Function,
    params: CodeGenParams,
}

impl<'a, 'b, D: IrDatabase> BodyIrGenerator<'a, 'b, D> {
    pub fn new(
        db: &'a D,
        module: &'a Module,
        hir_function: hir::Function,
        ir_function: FunctionValue,
        function_map: &'a HashMap<hir::Function, FunctionValue>,
        dispatch_table: &'b DispatchTable,
        params: CodeGenParams,
    ) -> Self {
        // Get the type information from the `hir::Function`
        let body = hir_function.body(db);
        let infer = hir_function.infer(db);

        // Construct a builder for the IR function
        let context = module.get_context();
        let builder = context.create_builder();
        let body_ir = context.append_basic_block(&ir_function, "body");
        builder.position_at_end(&body_ir);

        BodyIrGenerator {
            db,
            module,
            body,
            infer,
            builder,
            fn_value: ir_function,
            pat_to_param: HashMap::default(),
            pat_to_local: HashMap::default(),
            pat_to_name: HashMap::default(),
            function_map,
            dispatch_table,
            active_loop: None,
            hir_function,
            params,
        }
    }

    /// Generates IR for the body of the function.
    pub fn gen_fn_body(&mut self) {
        // Iterate over all parameters and their type and store them so we can reference them
        // later in code.
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
                    // Wildcard patterns cannot be referenced from code. So nothing to do.
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

        // Construct a return statement from the returned value of the body if a return is expected
        // in the first place. If the return type of the body is `never` there is no need to
        // generate a return statement.
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
        let args: Vec<BasicValueEnum> = fn_sig
            .params()
            .iter()
            .enumerate()
            .map(|(idx, ty)| {
                let param = self.fn_value.get_nth_param(idx as u32).unwrap();
                self.opt_deref_value(ty.clone(), param)
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
                    if hir_struct.data(self.db).memory_kind == hir::StructMemoryKind::Value {
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

    /// Generates IR for the specified expression. Dependending on the type of expression an IR
    /// value is returned.
    fn gen_expr(&mut self, expr: ExprId) -> Option<inkwell::values::BasicValueEnum> {
        let body = self.body.clone();
        match &body[expr] {
            Expr::Block {
                ref statements,
                tail,
            } => self.gen_block(expr, statements, *tail),
            Expr::Path(ref p) => {
                let resolver = hir::resolver_for_expr(self.body.clone(), self.db, expr);
                Some(self.gen_path_expr(p, expr, &resolver))
            }
            Expr::Literal(lit) => Some(self.gen_literal(lit)),
            Expr::RecordLit { fields, .. } => Some(self.gen_record_lit(expr, fields)),
            Expr::BinaryOp { lhs, rhs, op } => {
                self.gen_binary_op(expr, *lhs, *rhs, op.expect("missing op"))
            }
            Expr::Call {
                ref callee,
                ref args,
            } => {
                // Get the callable definition from the map
                match self.infer[*callee].as_callable_def() {
                    Some(hir::CallableDef::Function(def)) => {
                        // Get all the arguments
                        let args: Vec<BasicValueEnum> = args
                            .iter()
                            .map(|expr| self.gen_expr(*expr).expect("expected a value"))
                            .collect();

                        self.gen_call(def, &args).try_as_basic_value().left()
                    }
                    Some(hir::CallableDef::Struct(_)) => Some(self.gen_named_tuple_lit(expr, args)),
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
            } => {
                let ptr = self.gen_field(expr, *receiver_expr, name);
                let value = self.builder.build_load(ptr, &name.to_string());
                Some(value)
            }
            _ => unimplemented!("unimplemented expr type {:?}", &body[expr]),
        }
    }

    /// Generates an IR value that represents the given `Literal`.
    fn gen_literal(&mut self, lit: &Literal) -> BasicValueEnum {
        match lit {
            Literal::Int(v) => self
                .module
                .get_context()
                .i64_type()
                .const_int(unsafe { mem::transmute::<i64, u64>(*v) }, true)
                .into(),

            Literal::Float(v) => self
                .module
                .get_context()
                .f64_type()
                .const_float(*v as f64)
                .into(),

            Literal::Bool(value) => {
                let ty = self.module.get_context().bool_type();
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
    fn gen_empty(&mut self) -> BasicValueEnum {
        self.module.get_context().const_struct(&[], false).into()
    }

    /// Allocate a struct literal either on the stack or the heap based on the type of the struct.
    fn gen_struct_alloc(
        &mut self,
        hir_struct: hir::Struct,
        args: Vec<BasicValueEnum>,
    ) -> BasicValueEnum {
        // Construct the struct literal
        let struct_ty = self.db.struct_ty(hir_struct);
        let mut value: AggregateValueEnum = struct_ty.get_undef().into();
        for (i, arg) in args.into_iter().enumerate() {
            value = self
                .builder
                .build_insert_value(value, arg, i as u32, "init")
                .expect("Failed to initialize struct field.");
        }
        let struct_lit = value.into_struct_value();

        match hir_struct.data(self.db).memory_kind {
            hir::StructMemoryKind::Value => struct_lit.into(),
            hir::StructMemoryKind::GC => {
                // TODO: Root memory in GC
                self.gen_struct_alloc_on_heap(hir_struct, struct_lit)
            }
        }
    }

    fn gen_struct_alloc_on_heap(
        &mut self,
        hir_struct: hir::Struct,
        struct_lit: StructValue,
    ) -> BasicValueEnum {
        let struct_ir_ty = self.db.struct_ty(hir_struct);
        let malloc_fn_ptr = self
            .dispatch_table
            .gen_intrinsic_lookup(&self.builder, &intrinsics::malloc);
        let mem_ptr = self
            .builder
            .build_call(
                malloc_fn_ptr,
                &[
                    struct_ir_ty.size_of().unwrap().into(),
                    struct_ir_ty.get_alignment().into(),
                ],
                "malloc",
            )
            .try_as_basic_value()
            .left()
            .unwrap();
        let struct_ptr = self
            .builder
            .build_bitcast(
                mem_ptr,
                struct_ir_ty.ptr_type(AddressSpace::Generic),
                &hir_struct.name(self.db).to_string(),
            )
            .into_pointer_value();
        self.builder.build_store(struct_ptr, struct_lit);
        struct_ptr.into()
    }

    /// Generates IR for a record literal, e.g. `Foo { a: 1.23, b: 4 }`
    fn gen_record_lit(
        &mut self,
        type_expr: ExprId,
        fields: &[hir::RecordLitField],
    ) -> BasicValueEnum {
        let struct_ty = self.infer[type_expr].clone();
        let hir_struct = struct_ty.as_struct().unwrap(); // Can only really get here if the type is a struct
        let fields: Vec<BasicValueEnum> = fields
            .iter()
            .map(|field| self.gen_expr(field.expr).expect("expected a field value"))
            .collect();

        self.gen_struct_alloc(hir_struct, fields)
    }

    /// Generates IR for a named tuple literal, e.g. `Foo(1.23, 4)`
    fn gen_named_tuple_lit(&mut self, type_expr: ExprId, args: &[ExprId]) -> BasicValueEnum {
        let struct_ty = self.infer[type_expr].clone();
        let hir_struct = struct_ty.as_struct().unwrap(); // Can only really get here if the type is a struct
        let args: Vec<BasicValueEnum> = args
            .iter()
            .map(|expr| self.gen_expr(*expr).expect("expected a field value"))
            .collect();

        self.gen_struct_alloc(hir_struct, args)
    }

    /// Generates IR for a unit struct literal, e.g `Foo`
    fn gen_unit_struct_lit(&mut self, type_expr: ExprId) -> BasicValueEnum {
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
    ) -> Option<BasicValueEnum> {
        for statement in statements.iter() {
            match statement {
                Statement::Let {
                    pat, initializer, ..
                } => {
                    self.gen_let_statement(*pat, *initializer);
                }
                Statement::Expr(expr) => {
                    // No need to generate code after a statement that has a `never` return type.
                    self.gen_expr(*expr)?;
                }
            };
        }
        tail.and_then(|expr| self.gen_expr(expr))
            .or_else(|| Some(self.gen_empty()))
    }

    /// Constructs a builder that should be used to emit an `alloca` instruction. These instructions
    /// should be at the start of the IR.
    fn new_alloca_builder(&self) -> Builder {
        let temp_builder = Builder::create();
        let block = self
            .fn_value
            .get_first_basic_block()
            .expect("at this stage there must be a block");
        if let Some(first_instruction) = block.get_first_instruction() {
            temp_builder.position_before(&first_instruction);
        } else {
            temp_builder.position_at_end(&block);
        }
        temp_builder
    }

    /// Generate IR for a let statement: `let a:int = 3`
    fn gen_let_statement(&mut self, pat: PatId, initializer: Option<ExprId>) {
        let initializer = initializer.and_then(|expr| self.gen_expr(expr));

        match &self.body[pat] {
            Pat::Bind { name } => {
                let builder = self.new_alloca_builder();
                let pat_ty = self.infer[pat].clone();
                let ty = try_convert_any_to_basic(self.db.type_ir(
                    pat_ty.clone(),
                    CodeGenParams {
                        make_marshallable: false,
                    },
                ))
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
    }

    /// Generates IR for looking up a certain path expression.
    fn gen_path_expr(
        &mut self,
        path: &Path,
        expr: ExprId,
        resolver: &Resolver,
    ) -> inkwell::values::BasicValueEnum {
        let resolution = resolver
            .resolve_path_without_assoc_items(self.db, path)
            .take_values()
            .expect("unknown path");

        match resolution {
            Resolution::LocalBinding(pat) => {
                if let Some(param) = self.pat_to_param.get(&pat) {
                    *param
                } else if let Some(ptr) = self.pat_to_local.get(&pat) {
                    let name = self.pat_to_name.get(&pat).expect("could not find pat name");
                    self.builder.build_load(*ptr, &name)
                } else {
                    unreachable!("could not find the pattern..");
                }
            }
            Resolution::Def(hir::ModuleDef::Struct(_)) => self.gen_unit_struct_lit(expr),
            Resolution::Def(_) => panic!("no support for module definitions"),
        }
    }

    /// Given an expression and the type of the expression, optionally dereference the value.
    fn opt_deref_value(&mut self, ty: hir::Ty, value: BasicValueEnum) -> BasicValueEnum {
        match ty {
            hir::Ty::Apply(hir::ApplicationTy {
                ctor: hir::TypeCtor::Struct(s),
                ..
            }) => match s.data(self.db).memory_kind {
                hir::StructMemoryKind::GC => {
                    self.builder.build_load(value.into_pointer_value(), "deref")
                }
                hir::StructMemoryKind::Value => {
                    if self.params.make_marshallable {
                        self.builder.build_load(value.into_pointer_value(), "deref")
                    } else {
                        value
                    }
                }
            },
            _ => value,
        }
    }

    /// Generates IR for looking up a certain path expression.
    fn gen_path_place_expr(
        &self,
        path: &Path,
        _expr: ExprId,
        resolver: &Resolver,
    ) -> inkwell::values::PointerValue {
        let resolution = resolver
            .resolve_path_without_assoc_items(self.db, path)
            .take_values()
            .expect("unknown path");

        match resolution {
            Resolution::LocalBinding(pat) => *self
                .pat_to_local
                .get(&pat)
                .expect("unresolved local binding"),
            Resolution::Def(_) => panic!("no support for module definitions"),
        }
    }

    /// Generates IR to calculate a binary operation between two expressions.
    fn gen_binary_op(
        &mut self,
        _tgt_expr: ExprId,
        lhs: ExprId,
        rhs: ExprId,
        op: BinaryOp,
    ) -> Option<BasicValueEnum> {
        let lhs_type = self.infer[lhs].clone();
        let rhs_type = self.infer[rhs].clone();
        match lhs_type.as_simple() {
            Some(TypeCtor::Float(_ty)) => self.gen_binary_op_float(lhs, rhs, op),
            Some(TypeCtor::Int(ty)) => self.gen_binary_op_int(lhs, rhs, op, ty.signedness),
            _ => unimplemented!(
                "unimplemented operation {0}op{1}",
                lhs_type.display(self.db),
                rhs_type.display(self.db)
            ),
        }
    }

    /// Generates IR to calculate a binary operation between two floating point values.
    fn gen_binary_op_float(
        &mut self,
        lhs_expr: ExprId,
        rhs_expr: ExprId,
        op: BinaryOp,
    ) -> Option<BasicValueEnum> {
        let lhs = self
            .gen_expr(lhs_expr)
            .map(|value| self.opt_deref_value(self.infer[lhs_expr].clone(), value))
            .expect("no lhs value")
            .into_float_value();
        let rhs = self
            .gen_expr(rhs_expr)
            .map(|value| self.opt_deref_value(self.infer[rhs_expr].clone(), value))
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
                let place = self.gen_place_expr(lhs_expr);
                self.builder.build_store(place, rhs);
                Some(self.gen_empty())
            }
            _ => unimplemented!("Operator {:?} is not implemented for float", op),
        }
    }

    /// Generates IR to calculate a binary operation between two integer values.
    fn gen_binary_op_int(
        &mut self,
        lhs_expr: ExprId,
        rhs_expr: ExprId,
        op: BinaryOp,
        signedness: hir::Signedness,
    ) -> Option<BasicValueEnum> {
        let lhs = self
            .gen_expr(lhs_expr)
            .map(|value| self.opt_deref_value(self.infer[lhs_expr].clone(), value))
            .expect("no lhs value")
            .into_int_value();
        let rhs = self
            .gen_expr(rhs_expr)
            .map(|value| self.opt_deref_value(self.infer[lhs_expr].clone(), value))
            .expect("no rhs value")
            .into_int_value();
        match op {
            BinaryOp::ArithOp(op) => Some(self.gen_arith_bin_op_int(lhs, rhs, op).into()),
            BinaryOp::CmpOp(op) => {
                let (name, predicate) = match op {
                    CmpOp::Eq { negated: false } => ("eq", IntPredicate::EQ),
                    CmpOp::Eq { negated: true } => ("neq", IntPredicate::NE),
                    CmpOp::Ord {
                        ordering: Ordering::Less,
                        strict: false,
                    } => (
                        "lesseq",
                        match signedness {
                            hir::Signedness::Signed => IntPredicate::SLE,
                            hir::Signedness::Unsigned => IntPredicate::ULE,
                        },
                    ),
                    CmpOp::Ord {
                        ordering: Ordering::Less,
                        strict: true,
                    } => (
                        "less",
                        match signedness {
                            hir::Signedness::Signed => IntPredicate::SLT,
                            hir::Signedness::Unsigned => IntPredicate::ULT,
                        },
                    ),
                    CmpOp::Ord {
                        ordering: Ordering::Greater,
                        strict: false,
                    } => (
                        "greatereq",
                        match signedness {
                            hir::Signedness::Signed => IntPredicate::SGE,
                            hir::Signedness::Unsigned => IntPredicate::UGE,
                        },
                    ),
                    CmpOp::Ord {
                        ordering: Ordering::Greater,
                        strict: true,
                    } => (
                        "greater",
                        match signedness {
                            hir::Signedness::Signed => IntPredicate::SGT,
                            hir::Signedness::Unsigned => IntPredicate::UGT,
                        },
                    ),
                };
                Some(
                    self.builder
                        .build_int_compare(predicate, lhs, rhs, name)
                        .into(),
                )
            }
            BinaryOp::Assignment { op } => {
                let rhs = match op {
                    Some(op) => self.gen_arith_bin_op_int(lhs, rhs, op),
                    None => rhs,
                };
                let place = self.gen_place_expr(lhs_expr);
                self.builder.build_store(place, rhs);
                Some(self.gen_empty())
            }
            _ => unreachable!(format!("Operator {:?} is not implemented for integer", op)),
        }
    }

    fn gen_arith_bin_op_int(&mut self, lhs: IntValue, rhs: IntValue, op: ArithOp) -> IntValue {
        match op {
            ArithOp::Add => self.builder.build_int_add(lhs, rhs, "add"),
            ArithOp::Subtract => self.builder.build_int_sub(lhs, rhs, "sub"),
            ArithOp::Divide => self.builder.build_int_signed_div(lhs, rhs, "div"),
            ArithOp::Multiply => self.builder.build_int_mul(lhs, rhs, "mul"),
        }
    }

    fn gen_arith_bin_op_float(
        &mut self,
        lhs: FloatValue,
        rhs: FloatValue,
        op: ArithOp,
    ) -> FloatValue {
        match op {
            ArithOp::Add => self.builder.build_float_add(lhs, rhs, "add"),
            ArithOp::Subtract => self.builder.build_float_sub(lhs, rhs, "sub"),
            ArithOp::Divide => self.builder.build_float_div(lhs, rhs, "div"),
            ArithOp::Multiply => self.builder.build_float_mul(lhs, rhs, "mul"),
        }
    }

    /// Given an expression generate code that results in a memory address that can be used for
    /// other place operations.
    fn gen_place_expr(&mut self, expr: ExprId) -> PointerValue {
        let body = self.body.clone();
        match &body[expr] {
            Expr::Path(ref p) => {
                let resolver = hir::resolver_for_expr(self.body.clone(), self.db, expr);
                self.gen_path_place_expr(p, expr, &resolver)
            }
            Expr::Field {
                expr: receiver_expr,
                name,
            } => self.gen_field(expr, *receiver_expr, name),
            _ => unreachable!("invalid place expression"),
        }
    }

    fn should_use_dispatch_table(&self) -> bool {
        // FIXME: When we use the dispatch table, generated wrappers have infinite recursion
        !self.params.make_marshallable
    }

    /// Generates IR for a function call.
    fn gen_call(&mut self, function: hir::Function, args: &[BasicValueEnum]) -> CallSiteValue {
        if self.should_use_dispatch_table() {
            let ptr_value =
                self.dispatch_table
                    .gen_function_lookup(self.db, &self.builder, function);
            self.builder
                .build_call(ptr_value, &args, &function.name(self.db).to_string())
        } else {
            let llvm_function = self
                .function_map
                .get(&function)
                .expect("missing function value for hir function");
            self.builder
                .build_call(*llvm_function, &args, &function.name(self.db).to_string())
        }
    }

    /// Generates IR for an if statement.
    fn gen_if(
        &mut self,
        _expr: ExprId,
        condition: ExprId,
        then_branch: ExprId,
        else_branch: Option<ExprId>,
    ) -> Option<inkwell::values::BasicValueEnum> {
        // Generate IR for the condition
        let condition_ir = self
            .gen_expr(condition)
            .map(|value| self.opt_deref_value(self.infer[condition].clone(), value))?
            .into_int_value();

        // Generate the code blocks to branch to
        let context = self.module.get_context();
        let mut then_block = context.append_basic_block(&self.fn_value, "then");
        let else_block_and_expr = match &else_branch {
            Some(else_branch) => Some((
                context.append_basic_block(&self.fn_value, "else"),
                else_branch,
            )),
            None => None,
        };
        let merge_block = context.append_basic_block(&self.fn_value, "if_merge");

        // Build the actual branching IR for the if statement
        let else_block = else_block_and_expr
            .as_ref()
            .map(|e| &e.0)
            .unwrap_or(&merge_block);
        self.builder
            .build_conditional_branch(condition_ir, &then_block, else_block);

        // Fill the then block
        self.builder.position_at_end(&then_block);
        let then_block_ir = self.gen_expr(then_branch);
        if !self.infer[then_branch].is_never() {
            self.builder.build_unconditional_branch(&merge_block);
        }
        then_block = self.builder.get_insert_block().unwrap();

        // Fill the else block, if it exists and get the result back
        let else_ir_and_block = if let Some((else_block, else_branch)) = else_block_and_expr {
            else_block
                .move_after(&then_block)
                .expect("programmer error, then_block is invalid");
            self.builder.position_at_end(&else_block);
            let result_ir = self.gen_expr(*else_branch);
            if !self.infer[*else_branch].is_never() {
                self.builder.build_unconditional_branch(&merge_block);
            }
            result_ir.map(|res| (res, self.builder.get_insert_block().unwrap()))
        } else {
            None
        };

        // Create merge block
        let current_block = self.builder.get_insert_block().unwrap();
        merge_block.move_after(&current_block).unwrap();
        self.builder.position_at_end(&merge_block);

        // Construct phi block if a value was returned
        if let Some(then_block_ir) = then_block_ir {
            if let Some((else_block_ir, else_block)) = else_ir_and_block {
                let phi = self.builder.build_phi(then_block_ir.get_type(), "iftmp");
                phi.add_incoming(&[(&then_block_ir, &then_block), (&else_block_ir, &else_block)]);
                Some(phi.as_basic_value())
            } else {
                Some(then_block_ir)
            }
        } else {
            Some(self.gen_empty())
        }
    }

    fn gen_return(&mut self, _expr: ExprId, ret_expr: Option<ExprId>) -> Option<BasicValueEnum> {
        let ret_value = ret_expr.and_then(|expr| self.gen_expr(expr));

        // Construct a return statement from the returned value of the body
        if let Some(value) = ret_value {
            self.builder.build_return(Some(&value));
        } else {
            self.builder.build_return(None);
        }

        None
    }

    fn gen_break(&mut self, _expr: ExprId, break_expr: Option<ExprId>) -> Option<BasicValueEnum> {
        let break_value = break_expr.and_then(|expr| self.gen_expr(expr));
        let loop_info = self.active_loop.as_mut().unwrap();
        if let Some(break_value) = break_value {
            loop_info
                .break_values
                .push((break_value, self.builder.get_insert_block().unwrap()));
        }
        self.builder
            .build_unconditional_branch(&loop_info.exit_block);
        None
    }

    fn gen_loop_block_expr(
        &mut self,
        block: ExprId,
        exit_block: BasicBlock,
    ) -> (
        BasicBlock,
        Vec<(BasicValueEnum, BasicBlock)>,
        Option<BasicValueEnum>,
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
    ) -> Option<BasicValueEnum> {
        let context = self.module.get_context();
        let cond_block = context.append_basic_block(&self.fn_value, "whilecond");
        let loop_block = context.append_basic_block(&self.fn_value, "while");
        let exit_block = context.append_basic_block(&self.fn_value, "afterwhile");

        // Insert an explicit fall through from the current block to the condition check
        self.builder.build_unconditional_branch(&cond_block);

        // Generate condition block
        self.builder.position_at_end(&cond_block);
        let condition_ir = self
            .gen_expr(condition_expr)
            .map(|value| self.opt_deref_value(self.infer[condition_expr].clone(), value));
        if let Some(condition_ir) = condition_ir {
            self.builder.build_conditional_branch(
                condition_ir.into_int_value(),
                &loop_block,
                &exit_block,
            );
        } else {
            // If the condition doesn't return a value, we also immediately return without a value.
            // This can happen if the expression is a `never` expression.
            return None;
        }

        // Generate loop block
        self.builder.position_at_end(&loop_block);
        let (exit_block, _, value) = self.gen_loop_block_expr(body_expr, exit_block);
        if value.is_some() {
            self.builder.build_unconditional_branch(&cond_block);
        }

        // Generate exit block
        self.builder.position_at_end(&exit_block);

        Some(self.gen_empty())
    }

    fn gen_loop(&mut self, _expr: ExprId, body_expr: ExprId) -> Option<BasicValueEnum> {
        let context = self.module.get_context();
        let loop_block = context.append_basic_block(&self.fn_value, "loop");
        let exit_block = context.append_basic_block(&self.fn_value, "exit");

        // Insert an explicit fall through from the current block to the loop
        self.builder.build_unconditional_branch(&loop_block);

        // Generate the body of the loop
        self.builder.position_at_end(&loop_block);
        let (exit_block, break_values, value) = self.gen_loop_block_expr(body_expr, exit_block);
        if value.is_some() {
            self.builder.build_unconditional_branch(&loop_block);
        }

        // Move the builder to the exit block
        self.builder.position_at_end(&exit_block);

        if !break_values.is_empty() {
            let (value, _) = break_values.first().unwrap();
            let phi = self.builder.build_phi(value.get_type(), "exit");
            for (ref value, ref block) in break_values {
                phi.add_incoming(&[(value, block)])
            }
            Some(phi.as_basic_value())
        } else {
            None
        }
    }

    fn gen_field(&mut self, _expr: ExprId, receiver_expr: ExprId, name: &Name) -> PointerValue {
        let hir_struct = self.infer[receiver_expr]
            .as_struct()
            .expect("expected a struct");

        let field_idx = hir_struct
            .field(self.db, name)
            .expect("expected a struct field")
            .id()
            .into_raw()
            .into();

        let receiver_ptr = self.gen_place_expr(receiver_expr);
        let receiver_ptr = self
            .opt_deref_value(self.infer[receiver_expr].clone(), receiver_ptr.into())
            .into_pointer_value();
        unsafe {
            self.builder.build_struct_gep(
                receiver_ptr,
                field_idx,
                &format!("{}.{}", hir_struct.name(self.db), name),
            )
        }
    }
}

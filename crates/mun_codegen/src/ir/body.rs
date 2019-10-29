use crate::{ir::dispatch_table::DispatchTable, ir::try_convert_any_to_basic, IrDatabase};
use inkwell::{
    builder::Builder,
    module::Module,
    values::{BasicValueEnum, CallSiteValue, FloatValue, FunctionValue, IntValue},
    FloatPredicate, IntPredicate,
};
use mun_hir::{
    self as hir, ArithOp, BinaryOp, Body, CmpOp, Expr, ExprId, HirDisplay, InferenceResult,
    Literal, Ordering, Pat, PatId, Path, Resolution, Resolver, Statement, TypeCtor,
};
use std::{collections::HashMap, mem, sync::Arc};

mod name;
use name::OptName;

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
    function_map: &'a HashMap<mun_hir::Function, FunctionValue>,
    dispatch_table: &'b DispatchTable,
}

impl<'a, 'b, D: IrDatabase> BodyIrGenerator<'a, 'b, D> {
    pub fn new(
        db: &'a D,
        module: &'a Module,
        hir_function: hir::Function,
        ir_function: FunctionValue,
        function_map: &'a HashMap<mun_hir::Function, FunctionValue>,
        dispatch_table: &'b DispatchTable,
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
                    param.set_name(&name); // Assign a name to the IR value consistent with the code.
                    self.pat_to_param.insert(*pat, param);
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

        // Construct a return statement from the returned value of the body
        if let Some(value) = ret_value {
            self.builder.build_return(Some(&value));
        } else {
            self.builder.build_return(None);
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
                let resolver = mun_hir::resolver_for_expr(self.body.clone(), self.db, expr);
                Some(self.gen_path_expr(p, expr, &resolver))
            }
            Expr::Literal(lit) => Some(self.gen_literal(lit)),
            Expr::BinaryOp { lhs, rhs, op } => {
                Some(self.gen_binary_op(expr, *lhs, *rhs, op.expect("missing op")))
            }
            Expr::Call {
                ref callee,
                ref args,
            } => self.gen_call(*callee, &args).try_as_basic_value().left(),
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => self.gen_if(expr, *condition, *then_branch, *else_branch),
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

            Literal::Bool(value) => self
                .module
                .get_context()
                .bool_type()
                .const_int(if *value { 1 } else { 0 }, false)
                .into(),

            Literal::String(_) => unimplemented!("string literals are not implemented yet"),
        }
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
                    self.gen_expr(*expr);
                }
            };
        }
        tail.and_then(|expr| self.gen_expr(expr))
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
                let ty = try_convert_any_to_basic(self.db.type_ir(pat_ty.clone()))
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
        &self,
        path: &Path,
        _expr: ExprId,
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
    ) -> BasicValueEnum {
        let lhs_value = self.gen_expr(lhs).expect("no lhs value");
        let rhs_value = self.gen_expr(rhs).expect("no rhs value");
        let lhs_type = self.infer[lhs].clone();
        let rhs_type = self.infer[rhs].clone();

        match lhs_type.as_simple() {
            Some(TypeCtor::Float) => self.gen_binary_op_float(
                *lhs_value.as_float_value(),
                *rhs_value.as_float_value(),
                op,
            ),
            Some(TypeCtor::Int) => {
                self.gen_binary_op_int(*lhs_value.as_int_value(), *rhs_value.as_int_value(), op)
            }
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
        lhs: FloatValue,
        rhs: FloatValue,
        op: BinaryOp,
    ) -> BasicValueEnum {
        match op {
            BinaryOp::ArithOp(ArithOp::Add) => self.builder.build_float_add(lhs, rhs, "add").into(),
            BinaryOp::ArithOp(ArithOp::Subtract) => {
                self.builder.build_float_sub(lhs, rhs, "sub").into()
            }
            BinaryOp::ArithOp(ArithOp::Divide) => {
                self.builder.build_float_div(lhs, rhs, "div").into()
            }
            BinaryOp::ArithOp(ArithOp::Multiply) => {
                self.builder.build_float_mul(lhs, rhs, "mul").into()
            }
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
                self.builder
                    .build_float_compare(predicate, lhs, rhs, name)
                    .into()
            }
            _ => unimplemented!("Operator {:?} is not implemented for float", op),
        }
    }

    /// Generates IR to calculate a binary operation between two integer values.
    fn gen_binary_op_int(&mut self, lhs: IntValue, rhs: IntValue, op: BinaryOp) -> BasicValueEnum {
        match op {
            BinaryOp::ArithOp(ArithOp::Add) => self.builder.build_int_add(lhs, rhs, "add").into(),
            BinaryOp::ArithOp(ArithOp::Subtract) => {
                self.builder.build_int_sub(lhs, rhs, "sub").into()
            }
            BinaryOp::ArithOp(ArithOp::Divide) => {
                self.builder.build_int_signed_div(lhs, rhs, "div").into()
            }
            BinaryOp::ArithOp(ArithOp::Multiply) => {
                self.builder.build_int_mul(lhs, rhs, "mul").into()
            }
            BinaryOp::CmpOp(op) => {
                let (name, predicate) = match op {
                    CmpOp::Eq { negated: false } => ("eq", IntPredicate::EQ),
                    CmpOp::Eq { negated: true } => ("neq", IntPredicate::NE),
                    CmpOp::Ord {
                        ordering: Ordering::Less,
                        strict: false,
                    } => ("lesseq", IntPredicate::SLE),
                    CmpOp::Ord {
                        ordering: Ordering::Less,
                        strict: true,
                    } => ("less", IntPredicate::SLT),
                    CmpOp::Ord {
                        ordering: Ordering::Greater,
                        strict: false,
                    } => ("greatereq", IntPredicate::SGE),
                    CmpOp::Ord {
                        ordering: Ordering::Greater,
                        strict: true,
                    } => ("greater", IntPredicate::SGT),
                };
                self.builder
                    .build_int_compare(predicate, lhs, rhs, name)
                    .into()
            }
            _ => unreachable!(format!("Operator {:?} is not implemented for integer", op)),
        }
    }

    // TODO: Implement me!
    fn should_use_dispatch_table(&self) -> bool {
        true
    }

    /// Generates IR for a function call.
    fn gen_call(&mut self, callee: ExprId, args: &[ExprId]) -> CallSiteValue {
        // Get the function value from the map
        let function = self.infer[callee]
            .as_function_def()
            .expect("expected a function expression");

        // Get all the arguments
        let args: Vec<BasicValueEnum> = args
            .iter()
            .map(|expr| self.gen_expr(*expr).expect("expected a value"))
            .collect();

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
            .expect("condition must have a value")
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
        self.builder.build_unconditional_branch(&merge_block);
        then_block = self.builder.get_insert_block().unwrap();

        // Fill the else block, if it exists and get the result back
        let else_ir_and_block = if let Some((else_block, else_branch)) = else_block_and_expr {
            else_block.move_after(&then_block);
            self.builder.position_at_end(&else_block);
            let result_ir = self.gen_expr(*else_branch);
            self.builder.build_unconditional_branch(&merge_block);
            result_ir.map(|res| (res, self.builder.get_insert_block().unwrap()))
        } else {
            None
        };

        // Create merge block
        merge_block.move_after(&self.builder.get_insert_block().unwrap());
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
            None
        }
    }
}

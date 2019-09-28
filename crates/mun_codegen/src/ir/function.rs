use super::try_convert_any_to_basic;
use crate::IrDatabase;
use inkwell::builder::Builder;
use inkwell::values::{BasicValueEnum, FloatValue, InstructionOpcode, IntValue};
use inkwell::{module::Module, types::{AnyTypeEnum, BasicTypeEnum}, values::FunctionValue, OptimizationLevel};
use mun_hir::{
    ArithOp, BinaryOp, Body, Expr, ExprId, Function, HirDisplay, InferenceResult, Literal, Pat,
    PatId, Path, Resolution, Resolver, Statement, TypeCtor,
};
use std::collections::HashMap;
use std::mem;
use std::sync::Arc;
use inkwell::passes::{PassManagerBuilder, PassManager};

pub(crate) fn create_pass_manager(
    module: &Module,
    optimization_lvl: OptimizationLevel,
) -> PassManager<FunctionValue> {
    let pass_builder = PassManagerBuilder::create();
    pass_builder.set_optimization_level(optimization_lvl);

    let function_pass_manager = PassManager::create(module);
    pass_builder.populate_function_pass_manager(&function_pass_manager);
    function_pass_manager.initialize();

    function_pass_manager
}

pub(crate) fn gen_signature(db: &impl IrDatabase, f: Function, module: &Module) -> FunctionValue {
    let name = f.name(db).to_string();
    if let AnyTypeEnum::FunctionType(ty) = db.type_ir(f.ty(db)) {
        module.add_function(&name, ty, None)
    } else {
        panic!("not a function type")
    }
}

pub(crate) fn gen_body(
    db: &impl IrDatabase,
    f: Function,
    fun: FunctionValue,
    module: &Module,
) -> FunctionValue {
    let builder = db.context().create_builder();
    let body_ir = fun.append_basic_block("body");
    builder.position_at_end(&body_ir);

    let mut code_gen = BodyIrGenerator::new(db, module, f, fun, builder);

    code_gen.gen_fn_body();

    fun
}

#[derive(Debug)]
struct BodyIrGenerator<'a, D: IrDatabase> {
    db: &'a D,
    module: &'a Module,
    body: Arc<Body>,
    infer: Arc<InferenceResult>,
    builder: Builder,
    fn_value: FunctionValue,
    hir: Function,
    pat_to_param: HashMap<PatId, inkwell::values::BasicValueEnum>,
    pat_to_local: HashMap<PatId, inkwell::values::PointerValue>,
    pat_to_name: HashMap<PatId, String>,
}

impl<'a, D: IrDatabase> BodyIrGenerator<'a, D> {
    fn new(
        db: &'a D,
        module: &'a Module,
        f: Function,
        fn_value: FunctionValue,
        builder: Builder,
    ) -> Self {
        let body = f.body(db);
        let infer = f.infer(db);

        BodyIrGenerator {
            db,
            module,
            body,
            infer,
            builder,
            fn_value,
            hir: f,
            pat_to_param: HashMap::default(),
            pat_to_local: HashMap::default(),
            pat_to_name: HashMap::default(),
        }
    }

    fn gen_fn_body(&mut self) {
        for (i, (pat, _ty)) in self.body.params().iter().enumerate() {
            let body = self.body.clone(); // Avoid borrow issues
            match &body[*pat] {
                Pat::Bind { name } => {
                    let name = name.to_string();
                    let param = self.fn_value.get_nth_param(i as u32).unwrap();
                    param.set_name(&name);
                    self.pat_to_param.insert(*pat, param);
                    self.pat_to_name.insert(*pat, name);
                }
                Pat::Wild => {}
                Pat::Missing | Pat::Path(_) => unreachable!(),
            }
        }

        let ret_value = self.gen_expr(self.body.body_expr());
        if let Some(value) = ret_value {
            self.builder.build_return(Some(&value));
        } else {
            self.builder.build_return(None);
        }
    }

    fn gen_expr(&mut self, expr: ExprId) -> Option<inkwell::values::BasicValueEnum> {
        let body = self.body.clone();
        let mut value = match &body[expr] {
            &Expr::Block {
                ref statements,
                tail,
            } => {
                for statement in statements.iter() {
                    match statement {
                        Statement::Let {
                            pat, initializer, ..
                        } => {
                            self.gen_let_statement(pat, initializer);
                        }
                        Statement::Expr(expr) => {
                            self.gen_expr(*expr);
                        }
                    };
                }
                tail.and_then(|expr| self.gen_expr(expr))
            }
            Expr::Path(ref p) => {
                let resolver = mun_hir::resolver_for_expr(self.body.clone(), self.db, expr);
                Some(self.gen_path_expr(p, expr, &resolver))
            }
            Expr::Literal(lit) => match lit {
                Literal::Int(v) => Some(
                    self.module
                        .get_context()
                        .i64_type()
                        .const_int(unsafe { mem::transmute::<i64, u64>(*v) }, true)
                        .into(),
                ),
                Literal::Float(v) => Some(
                    self.module
                        .get_context()
                        .f64_type()
                        .const_float(*v as f64)
                        .into(),
                ),
                Literal::String(_) | Literal::Bool(_) => unreachable!(),
            },
            &Expr::BinaryOp { lhs, rhs, op } => {
                Some(self.gen_binary_op(lhs, rhs, op.expect("missing op")))
            }
            _ => None,
        };

        // Check expected type or perform implicit cast
        value = value.map(|value| {
            match (
                value.get_type(),
                try_convert_any_to_basic(self.db.type_ir(self.infer[expr].clone())),
            ) {
                (BasicTypeEnum::IntType(_), Some(target @ BasicTypeEnum::FloatType(_))) => self
                    .builder
                    .build_cast(InstructionOpcode::SIToFP, value, target, "implicit_cast")
                    .into(),
                (a, Some(b)) if a == b => value,
                _ => unreachable!("could not perform implicit cast"),
            }
        });

        value
    }

    fn new_alloca_builder(&self) -> Builder {
        let temp_builder = Builder::create();
        let block = self
            .builder
            .get_insert_block()
            .expect("at this stage there must be a block");
        if let Some(first_instruction) = block.get_first_instruction() {
            temp_builder.position_before(&first_instruction);
        } else {
            temp_builder.position_at_end(&block);
        }
        temp_builder
    }

    fn gen_let_statement(&mut self, pat: &PatId, initializer: &Option<ExprId>) {
        let initializer = initializer.and_then(|expr| self.gen_expr(expr));

        match &self.body[*pat] {
            Pat::Bind { name } => {
                let builder = self.new_alloca_builder();
                let ty = try_convert_any_to_basic(self.db.type_ir(self.infer[*pat].clone()))
                    .expect("expected basic type");
                let ptr = builder.build_alloca(ty, &name.to_string());
                self.pat_to_local.insert(*pat, ptr);
                self.pat_to_name.insert(*pat, name.to_string());
                if let Some(value) = initializer {
                    self.builder.build_store(ptr, value);
                };
            }
            Pat::Wild => {}
            Pat::Missing | Pat::Path(_) => unreachable!(),
        }
    }

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

    fn gen_binary_op(&mut self, lhs: ExprId, rhs: ExprId, op: BinaryOp) -> BasicValueEnum {
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
            _ => unreachable!(
                "Unsupported operation {0}op{1}",
                lhs_type.display(self.db),
                rhs_type.display(self.db)
            ),
        }
    }

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
            //                BinaryOp::Remainder => Some(self.gen_remainder(lhs, rhs)),
            //                BinaryOp::Power =>,
            //                BinaryOp::Assign,
            //                BinaryOp::AddAssign,
            //                BinaryOp::SubtractAssign,
            //                BinaryOp::DivideAssign,
            //                BinaryOp::MultiplyAssign,
            //                BinaryOp::RemainderAssign,
            //                BinaryOp::PowerAssign,
            _ => unreachable!(),
        }
    }

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
            //                BinaryOp::Remainder => Some(self.gen_remainder(lhs, rhs)),
            //                BinaryOp::Power =>,
            //                BinaryOp::Assign,
            //                BinaryOp::AddAssign,
            //                BinaryOp::SubtractAssign,
            //                BinaryOp::DivideAssign,
            //                BinaryOp::MultiplyAssign,
            //                BinaryOp::RemainderAssign,
            //                BinaryOp::PowerAssign,
            _ => unreachable!(),
        }
    }
}

trait OptName {
    fn get_name(&self) -> Option<&str>;
    fn set_name<T: AsRef<str>>(&self, name: T);
}

impl OptName for BasicValueEnum {
    fn get_name(&self) -> Option<&str> {
        match self {
            BasicValueEnum::ArrayValue(v) => v.get_name().to_str().ok(),
            BasicValueEnum::IntValue(v) => v.get_name().to_str().ok(),
            BasicValueEnum::FloatValue(v) => v.get_name().to_str().ok(),
            BasicValueEnum::PointerValue(v) => v.get_name().to_str().ok(),
            BasicValueEnum::StructValue(v) => v.get_name().to_str().ok(),
            BasicValueEnum::VectorValue(v) => v.get_name().to_str().ok(),
        }
    }

    fn set_name<T: AsRef<str>>(&self, name: T) {
        match self {
            BasicValueEnum::ArrayValue(v) => v.set_name(name.as_ref()),
            BasicValueEnum::IntValue(v) => v.set_name(name.as_ref()),
            BasicValueEnum::FloatValue(v) => v.set_name(name.as_ref()),
            BasicValueEnum::PointerValue(v) => v.set_name(name.as_ref()),
            BasicValueEnum::StructValue(v) => v.set_name(name.as_ref()),
            BasicValueEnum::VectorValue(v) => v.set_name(name.as_ref()),
        };
    }
}

use std::sync::Arc;

use c_codegen::{
    function::{self, FunctionParameter},
    r#type::{Function, Pointer},
    Block, ConcreteType, Expression, Statement, Value,
};
use mun_hir::HirDatabase;

use crate::{
    identifier::field_name_to_identifier, signatures::function_identifier, ty, CCodegenDatabase,
};

pub fn generate_definition(
    db: &dyn CCodegenDatabase,
    function: mun_hir::Function,
) -> function::Definition {
    let return_ty = function.ret_type(db.upcast());

    let name = function_identifier(db, function);
    let parameters = generate_parameters(db.upcast(), function, true);

    function::Definition {
        is_static: true,
        name,
        ty: Function {
            parameters,
            return_ty: crate::ty::generate(db.upcast(), &return_ty),
        },
        body: generate_body(db.upcast(), function),
    }
}

pub fn generate_parameters(
    db: &dyn HirDatabase,
    function: mun_hir::Function,
    with_parameter_names: bool,
) -> Vec<FunctionParameter> {
    function
        .params(db)
        .into_iter()
        .map(|param| {
            let ty = crate::ty::generate(db, param.ty());
            let name = if with_parameter_names {
                let name = param.name(db).map(|name| name.to_string());

                let identifier = field_name_to_identifier(
                    name.as_deref()
                        // Default to a wildcard "_" if no name is provided
                        .unwrap_or("_"),
                );

                Some(identifier)
            } else {
                None
            };

            FunctionParameter { ty, name }
        })
        .collect()
}

pub fn generate_pointer_type<'ty>(
    db: &dyn HirDatabase,
    parameters: impl Iterator<Item = &'ty mun_hir::Ty>,
    return_ty: &mun_hir::Ty,
) -> ConcreteType {
    ConcreteType::Pointer(Box::new(Pointer {
        pointer_ty: Function {
            parameters: parameters
                .map(|parameter| {
                    let ty = ty::generate(db, parameter);

                    FunctionParameter { ty, name: None }
                })
                .collect(),
            return_ty: ty::generate(db, return_ty),
        }
        .into(),
        is_const: false,
    }))
}

struct BodyGenerator<'db> {
    db: &'db dyn HirDatabase,
    body: Arc<mun_hir::Body>,
    infer: Arc<mun_hir::InferenceResult>,
}

impl<'db> BodyGenerator<'db> {
    fn new(db: &'db dyn HirDatabase, function: mun_hir::Function) -> Self {
        let body = function.body(db);
        let infer = function.infer(db);

        Self { db, body, infer }
    }

    fn generate_expression(&self, expr: mun_hir::ExprId) -> Expression {
        match &self.body[expr] {
            mun_hir::Expr::Call { callee, args } => {
                let callable_def = self.infer[*callee]
                    .as_callable_def()
                    .expect("expected a callable expression");

                match callable_def {
                    mun_hir::CallableDef::Function(fn_def) => todo!(),
                    mun_hir::CallableDef::Struct(_) => {
                        self.generate_named_tuple_literal(expr, args)
                    }
                }
            }
            mun_hir::Expr::MethodCall {
                receiver,
                method_name,
                args,
            } => todo!(),
            mun_hir::Expr::Path(path) => todo!(),
            mun_hir::Expr::If {
                condition,
                then_branch,
                else_branch,
            } => todo!(),
            mun_hir::Expr::UnaryOp { expr, op } => todo!(),
            mun_hir::Expr::BinaryOp { lhs, rhs, op } => todo!(),
            mun_hir::Expr::Index { base, index } => todo!(),
            mun_hir::Expr::Block { statements, tail } => todo!(),
            mun_hir::Expr::Return { expr } => todo!(),
            mun_hir::Expr::Break { expr } => todo!(),
            mun_hir::Expr::Loop { body } => todo!(),
            mun_hir::Expr::While { condition, body } => todo!(),
            mun_hir::Expr::RecordLit {
                type_id,
                fields,
                spread,
            } => todo!(),
            mun_hir::Expr::Field { expr, name } => todo!(),
            mun_hir::Expr::Array(items) => todo!(),
            mun_hir::Expr::Literal(literal) => todo!(),
            mun_hir::Expr::Missing => {
                unimplemented!("unimplemented expr type {:?}", &self.body[expr])
            }
        }
    }

    fn generate_named_tuple_literal(
        &self,
        tuple_expr: mun_hir::ExprId,
        args: &[mun_hir::ExprId],
    ) -> Expression {
        let ty = &self.infer[tuple_expr];
        let structure = ty.as_struct().expect("expected a struct type");
        let args = args
            .iter()
            .map(|&arg| self.generate_expression(arg))
            .collect::<Vec<_>>();

        self.generate_struct_alloc(structure, args)
    }

    fn generate_struct_alloc(
        &self,
        structure: mun_hir::Struct,
        args: Vec<Expression>,
    ) -> Expression {
        let fields = structure
            .fields(self.db)
            .into_iter()
            .map(|field| {
                let field_name = field.name(self.db).to_string();
                field_name_to_identifier(&field_name)
            })
            .zip(args)
            .collect();

        let literal = Value::Struct { fields };

        match structure.data(self.db.upcast()).memory_kind {
            mun_hir::StructMemoryKind::Gc => self.generate_struct_alloc_on_heap(literal),
            mun_hir::StructMemoryKind::Value => literal.into(),
        }
    }

    fn generate_struct_alloc_on_heap(&self, literal: Value) -> Expression {
        todo!()
    }
}

fn generate_body(db: &dyn HirDatabase, function: mun_hir::Function) -> Block {
    let body = function.body(db);
    let infer = function.infer(db);

    let mut statements = Vec::new();

    let generator = BodyGenerator::new(db, function);

    Block { statements }
}

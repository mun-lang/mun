use std::sync::Arc;

use c_codegen::{
    function::{self, FunctionParameter},
    operator::Assignment,
    r#type::{Function, Pointer},
    statement::Return,
    Block, ConcreteType, Expression, Identifier, Statement, Value, Variable, VariableDeclaration,
};
use mun_hir::{ExprId, HirDatabase, HirDisplay as _};

use crate::{
    identifier::field_name_to_identifier, signatures::function_identifier, ty, CCodegenDatabase,
};

// Is this safe enough? Should we mangle this to avoid collisions?
const RETURN_VARIABLE_NAME: &str = "__mun_return_value";

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

/// In C a generated expression consists of a variable that holds the result of the expression
/// and a block that contains the statements to initialize the expression.
struct GeneratedExpression {
    initializer: Block,
    ty: ConcreteType,
    variable: Variable,
}

impl GeneratedExpression {
    /// Converts this expression into a variable definition by adding the variable declaration and initializer block of this expression to the given statements. Returns the variable that holds the result of this expression.
    fn into_variable_definition(self, statements: &mut Vec<Statement>) -> Variable {
        // Generates the variable declaration, e.g.:
        // ```c
        // int __mun_return_value;
        // ```
        let declaration = VariableDeclaration {
            storage_class: None,
            ty: self.ty,
            identifier: self.variable.clone(),
            initializer: None,
        };

        statements.push(declaration.into());
        statements.push(Statement::Block(self.initializer));

        self.variable
    }
}

struct ExpressionGenerator {
    initializer: Vec<Statement>,
    ty: ConcreteType,
    variable: Variable,
}

impl ExpressionGenerator {
    pub fn new(expr: ExprId, ty: ConcreteType) -> Self {
        let variable_name = format!("__mun_expr_{}", expr.into_raw());

        Self {
            initializer: Vec::new(),
            ty,
            variable: Variable::new(&variable_name).expect("valid identifier"),
        }
    }

    /// Assigns the provided expression to the variable of this generator, e.g.:
    /// ```c
    /// __mun_expr_42 = 1337;
    /// ```
    pub fn assign_variable<E: Into<Expression>>(&mut self, expression: E) {
        self.initializer.push(
            Expression::from(Assignment {
                left: self.variable.clone().into(),
                right: expression.into(),
            })
            .into(),
        );
    }

    pub fn generate(self) -> GeneratedExpression {
        GeneratedExpression {
            initializer: Block {
                statements: self.initializer,
            },
            ty: self.ty,
            variable: self.variable,
        }
    }
}

impl<'db> BodyGenerator<'db> {
    fn new(db: &'db dyn HirDatabase, function: mun_hir::Function) -> Self {
        let body = function.body(db);
        let infer = function.infer(db);

        Self { db, body, infer }
    }

    fn generate_expression(&self, expr: mun_hir::ExprId) -> GeneratedExpression {
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
            mun_hir::Expr::Block { statements, tail } => {}
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
            mun_hir::Expr::Literal(literal) => {
                let ty = &self.infer[expr];
                let mut generator = ExpressionGenerator::new(expr, ty::generate(self.db, ty));

                let value = self.generate_literal(literal, ty);
                generator.assign_variable(value);

                generator.generate()
            }
            mun_hir::Expr::Missing => {
                unimplemented!("unimplemented expr type {:?}", &self.body[expr])
            }
        }
    }

    fn generate_literal(&self, literal: &mun_hir::Literal, ty: &mun_hir::Ty) -> Value {
        match literal {
            mun_hir::Literal::String(value) => Value::String(value.clone()),
            mun_hir::Literal::Bool(value) => Value::boolean(*value),
            mun_hir::Literal::Int(literal) => {
                let int_ty = match &ty.interned() {
                    mun_hir::TyKind::Int(int_ty) => int_ty,
                    _ => unreachable!(
                        "cannot generate code for anything but an integral type (literal type: {})",
                        ty.display(self.db)
                    ),
                };
                self.generate_literal_int(literal, int_ty)
            }
            mun_hir::Literal::Float(literal_float) => {
                let float_ty = match &ty.interned() {
                    mun_hir::TyKind::Float(float_ty) => float_ty,
                    _ => {
                        unreachable!("cannot generate code for anything but a floating point type (literal type: {})", ty.display(self.db))
                    }
                };
                self.generate_literal_float(literal_float, float_ty)
            }
        }
    }

    fn generate_literal_float(
        &self,
        literal: &mun_hir::LiteralFloat,
        float_ty: &mun_hir::FloatTy,
    ) -> Value {
        let value = literal.value;

        match float_ty.bitness {
            mun_hir::FloatBitness::X32 => Value::float(value),
            mun_hir::FloatBitness::X64 => Value::double(value),
        }
    }

    fn generate_literal_int(
        &self,
        literal: &mun_hir::LiteralInt,
        int_ty: &mun_hir::IntTy,
    ) -> Value {
        // TODO: Why are all literal values u128?
        let value = literal.value;

        match int_ty.signedness {
            mun_hir::Signedness::Signed => Value::signed_integer(
                i64::try_from(value).expect("signed integer literal is larger than i64"),
            ),
            mun_hir::Signedness::Unsigned => Value::unsigned_integer(
                u64::try_from(value).expect("unsigned integer literal is larger than u64"),
            ),
        }
    }

    fn generate_named_tuple_literal(
        &self,
        tuple_expr: mun_hir::ExprId,
        args: &[mun_hir::ExprId],
    ) -> GeneratedExpression {
        let ty = &self.infer[tuple_expr];

        let mut generator = ExpressionGenerator::new(tuple_expr, ty::generate(self.db, ty));

        let structure = ty.as_struct().expect("expected a struct type");
        let args = args
            .iter()
            .map(|&arg| {
                let generated = self.generate_expression(arg);
                generated.into_variable_definition(&mut generator.initializer)
            })
            .collect();

        let expression = self.generate_struct_alloc(structure, args);
        generator.assign_variable(expression);

        generator.generate()
    }

    fn generate_struct_alloc(&self, structure: mun_hir::Struct, args: Vec<Variable>) -> Expression {
        let fields = structure
            .fields(self.db)
            .into_iter()
            .map(|field| {
                let field_name = field.name(self.db).to_string();
                field_name_to_identifier(&field_name)
            })
            .zip(args.into_iter().map(Expression::from))
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

    let generated = generator.generate_expression(body.body_expr());
    let return_variable = generated.into_variable_definition(&mut statements);

    let return_type = &infer[body.body_expr()];
    if !return_type.is_never() {
        // Generates the return statement, e.g.:
        // ```c
        // return __mun_return_value;
        // ```
        statements.push(
            Return {
                expression: Some(Expression::Variable(return_variable)),
            }
            .into(),
        );
    }

    Block { statements }
}

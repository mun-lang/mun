use crate::{
    adt::StructKind,
    arena::map::ArenaMap,
    code_model::{DefWithBody, DefWithStruct, Struct},
    diagnostics::DiagnosticSink,
    expr,
    expr::{Body, Expr, ExprId, Literal, Pat, PatId, RecordLitField, Statement, UnaryOp},
    name_resolution::Namespace,
    resolve::{Resolution, Resolver},
    ty::infer::diagnostics::InferenceDiagnostic,
    ty::infer::type_variable::TypeVariableTable,
    ty::lower::LowerDiagnostic,
    ty::op,
    ty::{Ty, TypableDef},
    type_ref::TypeRefId,
    ApplicationTy, BinaryOp, Function, HirDatabase, Name, Path, TypeCtor,
};
use rustc_hash::FxHashSet;
use std::ops::Index;
use std::sync::Arc;

mod place_expr;
mod type_variable;
mod unify;

use crate::expr::{LiteralFloat, LiteralFloatKind, LiteralInt, LiteralIntKind};
use crate::ty::primitives::{FloatTy, IntTy};
use std::mem;
pub use type_variable::TypeVarId;

#[macro_export]
macro_rules! ty_app {
    ($ctor:pat, $param:pat) => {
        $crate::Ty::Apply($crate::ApplicationTy {
            ctor: $ctor,
            parameters: $param,
        })
    };
    ($ctor:pat) => {
        $crate::Ty::Apply($crate::ApplicationTy {
            ctor: $ctor,
            ..
        })
    };
}

mod coerce;

/// The result of type inference: A mapping from expressions and patterns to types.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct InferenceResult {
    pub(crate) type_of_expr: ArenaMap<ExprId, Ty>,
    pub(crate) type_of_pat: ArenaMap<PatId, Ty>,
    pub(crate) diagnostics: Vec<diagnostics::InferenceDiagnostic>,
}

impl Index<ExprId> for InferenceResult {
    type Output = Ty;
    fn index(&self, expr: ExprId) -> &Ty {
        self.type_of_expr.get(expr).unwrap_or(&Ty::Unknown)
    }
}

impl Index<PatId> for InferenceResult {
    type Output = Ty;
    fn index(&self, pat: PatId) -> &Ty {
        self.type_of_pat.get(pat).unwrap_or(&Ty::Unknown)
    }
}

impl InferenceResult {
    /// Adds all the `InferenceDiagnostic`s of the result to the `DiagnosticSink`.
    pub(crate) fn add_diagnostics(
        &self,
        db: &dyn HirDatabase,
        owner: Function,
        sink: &mut DiagnosticSink,
    ) {
        self.diagnostics
            .iter()
            .for_each(|it| it.add_to(db, owner, sink))
    }
}

/// The entry point of type inference. This method takes a body and infers the types of all the
/// expressions and patterns. Diagnostics are also reported and stored in the `InferenceResult`.
pub fn infer_query(db: &dyn HirDatabase, def: DefWithBody) -> Arc<InferenceResult> {
    let body = def.body(db);
    let resolver = def.resolver(db);
    let mut ctx = InferenceResultBuilder::new(db, body, resolver);

    match def {
        DefWithBody::Function(_) => ctx.infer_signature(),
    }

    ctx.infer_body();

    Arc::new(ctx.resolve_all())
}

/// Placeholders required during type inferencing. There are seperate values for integer and
/// floating-point types and for generic type variables. The first being used to distinguish
/// literals; e.g `100` can be represented by a lot of different integer types.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum InferTy {
    TypeVar(type_variable::TypeVarId),
    IntVar(type_variable::TypeVarId),
    FloatVar(type_variable::TypeVarId),
}

impl InferTy {
    fn to_inner(self) -> type_variable::TypeVarId {
        match self {
            InferTy::TypeVar(ty) | InferTy::IntVar(ty) | InferTy::FloatVar(ty) => ty,
        }
    }

    fn fallback_value(self) -> Ty {
        match self {
            InferTy::TypeVar(..) => Ty::Unknown,
            InferTy::IntVar(..) => Ty::simple(TypeCtor::Int(IntTy::i32())),
            InferTy::FloatVar(..) => Ty::simple(TypeCtor::Float(FloatTy::f64())),
        }
    }
}

enum ActiveLoop {
    Loop(Ty, Expectation),
    While,
    For,
}

/// The inference context contains all information needed during type inference.
struct InferenceResultBuilder<'a> {
    db: &'a dyn HirDatabase,
    body: Arc<Body>,
    resolver: Resolver,

    type_of_expr: ArenaMap<ExprId, Ty>,
    type_of_pat: ArenaMap<PatId, Ty>,
    diagnostics: Vec<InferenceDiagnostic>,

    type_variables: TypeVariableTable,

    /// Information on the current loop that we're processing (or None if we're not in a loop) the
    /// entry contains the current type of the loop statement (initially `never`) and the expected
    /// type of the loop expression. Both these values are updated when a break statement is
    /// encountered.
    active_loop: Option<ActiveLoop>,

    /// The return type of the function being inferred.
    return_ty: Ty,
}

impl<'a> InferenceResultBuilder<'a> {
    /// Construct a new `InferenceContext` from a `Body` and a `Resolver` for that body.
    fn new(db: &'a dyn HirDatabase, body: Arc<Body>, resolver: Resolver) -> Self {
        InferenceResultBuilder {
            type_of_expr: ArenaMap::default(),
            type_of_pat: ArenaMap::default(),
            diagnostics: Vec::default(),
            active_loop: None,
            type_variables: TypeVariableTable::default(),
            db,
            body,
            resolver,
            return_ty: Ty::Unknown, // set in collect_fn_signature
        }
    }

    /// Associate the given `ExprId` with the specified `Ty`.
    fn set_expr_type(&mut self, expr: ExprId, ty: Ty) {
        self.type_of_expr.insert(expr, ty);
    }

    /// Associate the given `PatId` with the specified `Ty`.
    fn set_pat_type(&mut self, pat: PatId, ty: Ty) {
        self.type_of_pat.insert(pat, ty);
    }

    /// Given a `TypeRefId`, resolve the reference to an actual `Ty`. If the the type could not
    /// be resolved an error is emitted and `Ty::Error` is returned.
    fn resolve_type(&mut self, type_ref: TypeRefId) -> Ty {
        // Try to resolve the type from the Hir
        let result = Ty::from_hir(
            self.db,
            // FIXME use right resolver for block
            &self.resolver,
            &self.body.type_refs(),
            type_ref,
        );

        // Convert the diagnostics from resolving the type reference
        for diag in result.diagnostics {
            let diag = match diag {
                LowerDiagnostic::UnresolvedType { id } => {
                    InferenceDiagnostic::UnresolvedType { id }
                }
            };
            self.diagnostics.push(diag);
        }

        result.ty
    }
}

impl<'a> InferenceResultBuilder<'a> {
    /// Collect all the parameter patterns from the body. After calling this method the `return_ty`
    /// will have a valid value, also all parameters are added inferred.
    fn infer_signature(&mut self) {
        let body = Arc::clone(&self.body); // avoid borrow checker problem

        // Iterate over all the parameters and associated types of the body and infer the types of
        // the parameters.
        for (pat, type_ref) in body.params().iter() {
            let ty = self.resolve_type(*type_ref);
            self.infer_pat(*pat, ty);
        }

        // Resolve the return type
        self.return_ty = self.resolve_type(body.ret_type())
    }

    /// Record the type of the specified pattern and all sub-patterns.
    fn infer_pat(&mut self, pat: PatId, ty: Ty) {
        let body = Arc::clone(&self.body); // avoid borrow checker problem
        #[allow(clippy::single_match)]
        match &body[pat] {
            Pat::Bind { .. } => {
                self.set_pat_type(pat, ty);
            }
            _ => {}
        }
    }

    /// Infer the types of all the expressions and sub-expressions in the body.
    fn infer_body(&mut self) {
        self.infer_expr_coerce(
            self.body.body_expr(),
            &Expectation::has_type(self.return_ty.clone()),
        );
    }

    /// Infers the type of the `tgt_expr`
    fn infer_expr(&mut self, tgt_expr: ExprId, expected: &Expectation) -> Ty {
        let ty = self.infer_expr_inner(tgt_expr, expected, &CheckParams::default());
        if !self.unify(&ty, &expected.ty) {
            self.diagnostics.push(InferenceDiagnostic::MismatchedTypes {
                expected: expected.ty.clone(),
                found: ty.clone(),
                id: tgt_expr,
            });
        };

        self.resolve_ty_as_far_as_possible(ty)
    }

    /// Infer type of expression with possibly implicit coerce to the expected type. Return the type
    /// after possible coercion. Adds a diagnostic message if coercion failed.
    fn infer_expr_coerce(&mut self, expr: ExprId, expected: &Expectation) -> Ty {
        let ty = self.infer_expr_inner(expr, expected, &CheckParams::default());
        self.coerce_expr_ty(expr, ty, expected)
    }

    /// Performs implicit coercion of the specified `Ty` to an expected type. Returns the type after
    /// possible coercion. Adds a diagnostic message if coercion failed.
    fn coerce_expr_ty(&mut self, expr: ExprId, ty: Ty, expected: &Expectation) -> Ty {
        let ty = if !self.coerce(&ty, &expected.ty) {
            self.diagnostics.push(InferenceDiagnostic::MismatchedTypes {
                expected: expected.ty.clone(),
                found: ty.clone(),
                id: expr,
            });
            ty
        } else if expected.ty == Ty::Unknown {
            ty
        } else {
            expected.ty.clone()
        };

        self.resolve_ty_as_far_as_possible(ty)
    }

    /// Infer the type of the given expression. Returns the type of the expression.
    fn infer_expr_inner(
        &mut self,
        tgt_expr: ExprId,
        expected: &Expectation,
        check_params: &CheckParams,
    ) -> Ty {
        let body = Arc::clone(&self.body); // avoid borrow checker problem
        let ty = match &body[tgt_expr] {
            Expr::Missing => Ty::Unknown,
            Expr::Path(p) => {
                // FIXME this could be more efficient...
                let resolver = expr::resolver_for_expr(self.body.clone(), self.db, tgt_expr);
                self.infer_path_expr(&resolver, p, tgt_expr, check_params)
                    .unwrap_or(Ty::Unknown)
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => self.infer_if(tgt_expr, &expected, *condition, *then_branch, *else_branch),
            Expr::BinaryOp { lhs, rhs, op } => match op {
                Some(op) => {
                    let lhs_expected = match op {
                        BinaryOp::LogicOp(..) => Expectation::has_type(Ty::simple(TypeCtor::Bool)),
                        _ => Expectation::none(),
                    };
                    let lhs_ty = self.infer_expr(*lhs, &lhs_expected);
                    if let BinaryOp::Assignment { op: _op } = op {
                        let resolver =
                            expr::resolver_for_expr(self.body.clone(), self.db, tgt_expr);
                        if !self.check_place_expression(&resolver, *lhs) {
                            self.diagnostics.push(InferenceDiagnostic::InvalidLHS {
                                id: tgt_expr,
                                lhs: *lhs,
                            })
                        }
                    };
                    let rhs_expected = op::binary_op_rhs_expectation(*op, lhs_ty.clone());
                    if lhs_ty != Ty::Unknown && rhs_expected == Ty::Unknown {
                        self.diagnostics
                            .push(InferenceDiagnostic::CannotApplyBinaryOp {
                                id: tgt_expr,
                                lhs: lhs_ty,
                                rhs: rhs_expected.clone(),
                            })
                    }
                    let rhs_ty = self.infer_expr(*rhs, &Expectation::has_type(rhs_expected));
                    op::binary_op_return_ty(*op, rhs_ty)
                }
                _ => Ty::Unknown,
            },
            Expr::Block { statements, tail } => self.infer_block(statements, *tail, expected),
            Expr::Call { callee: call, args } => self.infer_call(tgt_expr, *call, args, expected),
            Expr::Literal(lit) => match lit {
                Literal::String(_) => Ty::Unknown,
                Literal::Bool(_) => Ty::simple(TypeCtor::Bool),
                Literal::Int(LiteralInt {
                    kind: LiteralIntKind::Suffixed(suffix),
                    ..
                }) => Ty::simple(TypeCtor::Int(IntTy {
                    bitness: suffix.bitness,
                    signedness: suffix.signedness,
                })),
                Literal::Float(LiteralFloat {
                    kind: LiteralFloatKind::Suffixed(suffix),
                    ..
                }) => Ty::simple(TypeCtor::Float(FloatTy {
                    bitness: suffix.bitness,
                })),
                Literal::Int(LiteralInt {
                    kind: LiteralIntKind::Unsuffixed,
                    ..
                }) => self.type_variables.new_integer_var(),
                Literal::Float(LiteralFloat {
                    kind: LiteralFloatKind::Unsuffixed,
                    ..
                }) => self.type_variables.new_float_var(),
            },
            Expr::Return { expr } => {
                if let Some(expr) = expr {
                    self.infer_expr(*expr, &Expectation::has_type(self.return_ty.clone()));
                } else if self.return_ty != Ty::Empty {
                    self.diagnostics
                        .push(InferenceDiagnostic::ReturnMissingExpression { id: tgt_expr });
                }

                Ty::simple(TypeCtor::Never)
            }
            Expr::Break { expr } => self.infer_break(tgt_expr, *expr),
            Expr::Loop { body } => self.infer_loop_expr(tgt_expr, *body, expected),
            Expr::While { condition, body } => {
                self.infer_while_expr(tgt_expr, *condition, *body, expected)
            }
            Expr::RecordLit {
                type_id,
                fields,
                spread,
            } => {
                let ty = self.resolve_type(*type_id);
                let def_id = ty.as_struct();
                self.unify(&ty, &expected.ty);

                for (idx, field) in fields.iter().enumerate() {
                    let field_ty = def_id
                        .as_ref()
                        .and_then(|it| match it.field(self.db, &field.name) {
                            Some(field) => Some(field),
                            None => {
                                self.diagnostics.push(InferenceDiagnostic::NoSuchField {
                                    id: tgt_expr,
                                    field: idx,
                                });
                                None
                            }
                        })
                        .map_or(Ty::Unknown, |field| field.ty(self.db));
                    self.infer_expr_coerce(field.expr, &Expectation::has_type(field_ty));
                }
                if let Some(expr) = spread {
                    self.infer_expr(*expr, &Expectation::has_type(ty.clone()));
                }
                if let Some(s) = ty.as_struct() {
                    self.check_record_lit(tgt_expr, s, &fields);
                }
                ty
            }
            Expr::Field { expr, name } => {
                let receiver_ty = self.infer_expr(*expr, &Expectation::none());
                match receiver_ty {
                    ty_app!(TypeCtor::Struct(s)) => {
                        match s.field(self.db, name).map(|field| field.ty(self.db)) {
                            Some(field_ty) => field_ty,
                            None => {
                                self.diagnostics
                                    .push(InferenceDiagnostic::AccessUnknownField {
                                        id: tgt_expr,
                                        receiver_ty,
                                        name: name.clone(),
                                    });

                                Ty::Unknown
                            }
                        }
                    }
                    _ => {
                        self.diagnostics.push(InferenceDiagnostic::NoFields {
                            id: *expr,
                            found: receiver_ty,
                        });
                        Ty::Unknown
                    }
                }
            }
            Expr::UnaryOp { expr, op } => {
                let inner_ty =
                    self.infer_expr_inner(*expr, &Expectation::none(), &CheckParams::default());
                match op {
                    UnaryOp::Not => match &inner_ty {
                        Ty::Apply(ApplicationTy {
                            ctor: TypeCtor::Bool,
                            ..
                        })
                        | Ty::Apply(ApplicationTy {
                            ctor: TypeCtor::Int(_),
                            ..
                        })
                        | Ty::Infer(InferTy::IntVar(..)) => inner_ty,
                        _ => {
                            self.diagnostics
                                .push(InferenceDiagnostic::CannotApplyUnaryOp {
                                    id: *expr,
                                    ty: inner_ty,
                                });
                            Ty::Unknown
                        }
                    },
                    UnaryOp::Neg => match &inner_ty {
                        Ty::Apply(ApplicationTy {
                            ctor: TypeCtor::Float(_),
                            ..
                        })
                        | Ty::Apply(ApplicationTy {
                            ctor: TypeCtor::Int(_),
                            ..
                        })
                        | Ty::Infer(InferTy::IntVar(..))
                        | Ty::Infer(InferTy::FloatVar(..)) => inner_ty,
                        _ => {
                            self.diagnostics
                                .push(InferenceDiagnostic::CannotApplyUnaryOp {
                                    id: *expr,
                                    ty: inner_ty,
                                });
                            Ty::Unknown
                        }
                    },
                }
            } //            Expr::Block { statements: _, tail: _ } => {}
        };

        let ty = self.resolve_ty_as_far_as_possible(ty);
        self.set_expr_type(tgt_expr, ty.clone());
        ty
    }

    /// Inferences the type of an if statement.
    fn infer_if(
        &mut self,
        tgt_expr: ExprId,
        expected: &Expectation,
        condition: ExprId,
        then_branch: ExprId,
        else_branch: Option<ExprId>,
    ) -> Ty {
        self.infer_expr(
            condition,
            &Expectation::has_type(Ty::simple(TypeCtor::Bool)),
        );
        let then_ty = self.infer_expr_coerce(then_branch, expected);
        match else_branch {
            Some(else_branch) => {
                let else_ty = self.infer_expr_coerce(else_branch, expected);
                match self.coerce_merge_branch(&then_ty, &else_ty) {
                    Some(ty) => ty,
                    None => {
                        self.diagnostics
                            .push(InferenceDiagnostic::IncompatibleBranches {
                                id: tgt_expr,
                                then_ty: then_ty.clone(),
                                else_ty: else_ty.clone(),
                            });
                        then_ty
                    }
                }
            }
            None => {
                if !self.coerce(&then_ty, &Ty::Empty) {
                    self.diagnostics
                        .push(InferenceDiagnostic::MissingElseBranch {
                            id: tgt_expr,
                            then_ty,
                        })
                }
                Ty::Empty
            }
        }
    }

    /// Inferences the type of a call expression.
    fn infer_call(
        &mut self,
        tgt_expr: ExprId,
        callee: ExprId,
        args: &[ExprId],
        _expected: &Expectation,
    ) -> Ty {
        let callee_ty = self.infer_expr_inner(
            callee,
            &Expectation::none(),
            &CheckParams {
                is_unit_struct: false,
            },
        );

        match callee_ty {
            ty_app!(TypeCtor::Struct(s)) => {
                // Erroneously found either a unit struct or tuple struct literal
                let struct_data = s.data(self.db.upcast());
                self.diagnostics
                    .push(InferenceDiagnostic::MismatchedStructLit {
                        id: tgt_expr,
                        expected: struct_data.kind,
                        found: StructKind::Tuple,
                    });

                // Still derive subtypes
                for arg in args.iter() {
                    self.infer_expr(*arg, &Expectation::none());
                }

                callee_ty
            }
            ty_app!(TypeCtor::FnDef(def)) => {
                // Found either a tuple struct literal or function
                let sig = callee_ty.callable_sig(self.db).unwrap();
                let (param_tys, ret_ty) = (sig.params().to_vec(), sig.ret().clone());
                self.check_call_argument_count(
                    tgt_expr,
                    def.is_struct(),
                    args.len(),
                    param_tys.len(),
                );
                for (&arg, param_ty) in args.iter().zip(param_tys.iter()) {
                    self.infer_expr_coerce(arg, &Expectation::has_type(param_ty.clone()));
                }

                ret_ty
            }
            _ => {
                self.diagnostics
                    .push(InferenceDiagnostic::ExpectedFunction {
                        id: callee,
                        found: callee_ty,
                    });
                Ty::Unknown
            }
        }
    }

    /// Checks whether the specified struct type is a unit struct.
    fn check_unit_struct_lit(&mut self, tgt_expr: ExprId, expected: Struct) {
        let struct_data = expected.data(self.db.upcast());
        if struct_data.kind != StructKind::Unit {
            self.diagnostics
                .push(InferenceDiagnostic::MismatchedStructLit {
                    id: tgt_expr,
                    expected: struct_data.kind,
                    found: StructKind::Unit,
                });
        }
    }

    /// Checks whether the number of passed arguments matches the number of parameters of a
    /// callable definition.
    fn check_call_argument_count(
        &mut self,
        tgt_expr: ExprId,
        is_tuple_lit: bool,
        num_args: usize,
        num_params: usize,
    ) {
        if num_args != num_params {
            self.diagnostics.push(if is_tuple_lit {
                InferenceDiagnostic::FieldCountMismatch {
                    id: tgt_expr,
                    found: num_args,
                    expected: num_params,
                }
            } else {
                InferenceDiagnostic::ParameterCountMismatch {
                    id: tgt_expr,
                    found: num_args,
                    expected: num_params,
                }
            })
        }
    }

    // Checks whether the passed fields match the fields of a struct definition.
    fn check_record_lit(&mut self, tgt_expr: ExprId, expected: Struct, fields: &[RecordLitField]) {
        let struct_data = expected.data(self.db.upcast());
        if struct_data.kind != StructKind::Record {
            self.diagnostics
                .push(InferenceDiagnostic::MismatchedStructLit {
                    id: tgt_expr,
                    expected: struct_data.kind,
                    found: StructKind::Record,
                });
            return;
        }

        let lit_fields: FxHashSet<_> = fields.iter().map(|f| &f.name).collect();
        let missed_fields: Vec<Name> = struct_data
            .fields
            .iter()
            .filter_map(|(_f, d)| {
                let name = d.name.clone();
                if lit_fields.contains(&name) {
                    None
                } else {
                    Some(name)
                }
            })
            .collect();

        if !missed_fields.is_empty() {
            self.diagnostics.push(InferenceDiagnostic::MissingFields {
                id: tgt_expr,
                names: missed_fields,
            });
        }
    }

    fn infer_path_expr(
        &mut self,
        resolver: &Resolver,
        path: &Path,
        id: ExprId,
        check_params: &CheckParams,
    ) -> Option<Ty> {
        let resolution = match resolver
            .resolve_path_without_assoc_items(self.db, path)
            .take_values()
        {
            Some(resolution) => resolution,
            None => {
                self.diagnostics
                    .push(InferenceDiagnostic::UnresolvedValue { id: id.into() });
                return None;
            }
        };

        match resolution {
            Resolution::LocalBinding(pat) => {
                let ty = self.type_of_pat.get(pat)?.clone();
                //let ty = self.resolve_ty_as_possible(&mut vec![], ty);
                Some(ty)
            }
            Resolution::Def(def) => {
                let typable: Option<TypableDef> = def.into();
                let typable = typable?;
                let ty = self.db.type_for_def(typable, Namespace::Values);
                if check_params.is_unit_struct {
                    if let Some(s) = ty.as_struct() {
                        self.check_unit_struct_lit(id, s);
                    }
                }
                Some(ty)
            }
        }
    }

    fn resolve_all(mut self) -> InferenceResult {
        // FIXME resolve obligations as well (use Guidance if necessary)
        //let mut tv_stack = Vec::new();
        let mut expr_types = std::mem::take(&mut self.type_of_expr);
        for (expr, ty) in expr_types.iter_mut() {
            let was_unknown = ty == &mut Ty::Unknown;
            let resolved = self
                .type_variables
                .resolve_ty_completely(mem::replace(ty, Ty::Unknown));
            if !was_unknown && resolved == Ty::Unknown {
                self.report_expr_inference_failure(expr);
            }
            *ty = resolved;
        }
        let mut pat_types = std::mem::take(&mut self.type_of_pat);
        for (pat, ty) in pat_types.iter_mut() {
            let was_unknown = ty == &mut Ty::Unknown;
            let resolved = self
                .type_variables
                .resolve_ty_completely(mem::replace(ty, Ty::Unknown));
            if !was_unknown && resolved == Ty::Unknown {
                self.report_pat_inference_failure(pat);
            }
            *ty = resolved;
        }
        InferenceResult {
            //            method_resolutions: self.method_resolutions,
            //            field_resolutions: self.field_resolutions,
            //            variant_resolutions: self.variant_resolutions,
            //            assoc_resolutions: self.assoc_resolutions,
            type_of_expr: expr_types,
            type_of_pat: pat_types,
            diagnostics: self.diagnostics,
        }
    }

    fn resolve_struct(&mut self, path: Option<&Path>) -> (Ty, Option<DefWithStruct>) {
        let path = match path {
            Some(path) => path,
            None => return (Ty::Unknown, None),
        };
        let resolver = &self.resolver;
        let resolution = match resolver
            .resolve_path_without_assoc_items(self.db, &path)
            .take_types()
        {
            Some(resolution) => resolution,
            None => return (Ty::Unknown, None),
        };

        match resolution {
            Resolution::LocalBinding(pat) => {
                let ty = self
                    .type_of_pat
                    .get(pat)
                    .map_or(Ty::Unknown, |ty| ty.clone());
                //let ty = self.resolve_ty_as_possible(&mut vec![], ty);
                (ty, None)
            }
            Resolution::Def(def) => {
                if let Some(typable) = def.into() {
                    match typable {
                        TypableDef::Struct(s) => (s.ty(self.db), Some(s.into())),
                        TypableDef::BuiltinType(_) | TypableDef::Function(_) => (Ty::Unknown, None),
                    }
                } else {
                    unreachable!();
                }
            }
        }
    }

    fn infer_block(
        &mut self,
        statements: &[Statement],
        tail: Option<ExprId>,
        expected: &Expectation,
    ) -> Ty {
        let mut diverges = false;
        for stmt in statements {
            match stmt {
                Statement::Let {
                    pat,
                    type_ref,
                    initializer,
                } => {
                    let decl_ty = type_ref
                        .as_ref()
                        .map(|tr| self.resolve_type(*tr))
                        .unwrap_or(Ty::Unknown);
                    //let decl_ty = self.insert_type_vars(decl_ty);
                    let ty = if let Some(expr) = initializer {
                        self.infer_expr_coerce(*expr, &Expectation::has_type(decl_ty))
                    } else {
                        decl_ty
                    };

                    let ty = self.resolve_ty_as_far_as_possible(ty);
                    self.infer_pat(*pat, ty);
                }
                Statement::Expr(expr) => {
                    if let ty_app!(TypeCtor::Never) = self.infer_expr(*expr, &Expectation::none()) {
                        diverges = true;
                    };
                }
            }
        }
        let ty = if let Some(expr) = tail {
            // Perform coercion of the trailing expression unless the expression has a Never return
            // type because we want the block to get the Never type in that case.
            let ty = self.infer_expr_inner(expr, expected, &CheckParams::default());
            if let ty_app!(TypeCtor::Never) = ty {
                Ty::simple(TypeCtor::Never)
            } else {
                self.coerce_expr_ty(expr, ty, expected)
            }
        } else {
            Ty::Empty
        };

        if diverges {
            Ty::simple(TypeCtor::Never)
        } else {
            ty
        }
    }

    fn infer_break(&mut self, tgt_expr: ExprId, expr: Option<ExprId>) -> Ty {
        let expected = match &self.active_loop {
            Some(ActiveLoop::Loop(_, info)) => info.clone(),
            Some(_) => {
                if expr.is_some() {
                    self.diagnostics
                        .push(InferenceDiagnostic::BreakWithValueOutsideLoop { id: tgt_expr });
                }
                return Ty::simple(TypeCtor::Never);
            }
            None => {
                self.diagnostics
                    .push(InferenceDiagnostic::BreakOutsideLoop { id: tgt_expr });
                return Ty::simple(TypeCtor::Never);
            }
        };

        // Infer the type of the break expression
        let ty = if let Some(expr) = expr {
            self.infer_expr_inner(expr, &expected, &CheckParams::default())
        } else {
            Ty::Empty
        };

        // Verify that it matches what we expected
        let ty = if !self.unify(&ty, &expected.ty) {
            self.diagnostics.push(InferenceDiagnostic::MismatchedTypes {
                expected: expected.ty.clone(),
                found: ty,
                id: tgt_expr,
            });
            expected.ty
        } else {
            ty
        };

        // Update the expected type for the rest of the loop
        self.active_loop = Some(ActiveLoop::Loop(ty.clone(), Expectation::has_type(ty)));

        Ty::simple(TypeCtor::Never)
    }

    fn infer_loop_expr(&mut self, _tgt_expr: ExprId, body: ExprId, expected: &Expectation) -> Ty {
        if let ActiveLoop::Loop(ty, _) = self.infer_loop_block(
            body,
            ActiveLoop::Loop(Ty::simple(TypeCtor::Never), expected.clone()),
        ) {
            ty
        } else {
            panic!("returned active loop must be a loop")
        }
    }

    fn infer_loop_block(&mut self, body: ExprId, lp: ActiveLoop) -> ActiveLoop {
        let top_level_loop = std::mem::replace(&mut self.active_loop, Some(lp));

        // Infer the body of the loop
        self.infer_expr_coerce(body, &Expectation::has_type(Ty::Empty));

        // Take the result of the loop information and replace with top level loop
        std::mem::replace(&mut self.active_loop, top_level_loop).unwrap()
    }

    fn infer_while_expr(
        &mut self,
        _tgt_expr: ExprId,
        condition: ExprId,
        body: ExprId,
        _expected: &Expectation,
    ) -> Ty {
        self.infer_expr(
            condition,
            &Expectation::has_type(Ty::simple(TypeCtor::Bool)),
        );

        self.infer_loop_block(body, ActiveLoop::While);
        Ty::Empty
    }

    pub fn report_pat_inference_failure(&mut self, _pat: PatId) {
        //        self.diagnostics.push(InferenceDiagnostic::PatInferenceFailed {
        //            pat
        //        });
        // Currently this should never happen because we can only infer integer and floating-point
        // types which always have a fallback value.
        panic!("pattern failed inferencing");
    }

    pub fn report_expr_inference_failure(&mut self, _expr: ExprId) {
        //        self.diagnostics.push(InferenceDiagnostic::ExprInferenceFailed {
        //            expr
        //        });
        // Currently this should never happen because we can only infer integer and floating-point
        // types which always have a fallback value.
        panic!("expression failed inferencing");
    }
}

/// When inferring an expression, we propagate downward whatever type hint we
/// are able in the form of an `Expectation`.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Expectation {
    ty: Ty,
    // FIXME: In some cases, we need to be aware whether the expectation is that
    // the type match exactly what we passed, or whether it just needs to be
    // coercible to the expected type. See Expectation::rvalue_hint in rustc.
}

impl Expectation {
    /// The expectation that the type of the expression needs to equal the given
    /// type.
    fn has_type(ty: Ty) -> Self {
        Expectation { ty }
    }

    /// This expresses no expectation on the type.
    fn none() -> Self {
        Expectation { ty: Ty::Unknown }
    }

    fn is_none(&self) -> bool {
        self.ty == Ty::Unknown
    }
}

/// Parameters for toggling validation checks.
struct CheckParams {
    /// Checks whether a `Expr::Path` of type struct, is actually a unit struct
    is_unit_struct: bool,
}

impl Default for CheckParams {
    fn default() -> Self {
        Self {
            is_unit_struct: true,
        }
    }
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub(crate) enum ExprOrPatId {
    ExprId(ExprId),
    PatId(PatId),
}

impl From<ExprId> for ExprOrPatId {
    fn from(e: ExprId) -> Self {
        ExprOrPatId::ExprId(e)
    }
}

impl From<PatId> for ExprOrPatId {
    fn from(p: PatId) -> Self {
        ExprOrPatId::PatId(p)
    }
}

mod diagnostics {
    use crate::diagnostics::{
        AccessUnknownField, BreakOutsideLoop, BreakWithValueOutsideLoop, CannotApplyBinaryOp,
        CannotApplyUnaryOp, ExpectedFunction, FieldCountMismatch, IncompatibleBranch, InvalidLHS,
        LiteralOutOfRange, MismatchedStructLit, MismatchedType, MissingElseBranch, MissingFields,
        NoFields, NoSuchField, ParameterCountMismatch, ReturnMissingExpression,
    };
    use crate::{
        adt::StructKind,
        code_model::src::HasSource,
        diagnostics::{DiagnosticSink, UnresolvedType, UnresolvedValue},
        ty::infer::ExprOrPatId,
        type_ref::TypeRefId,
        ExprId, Function, HirDatabase, IntTy, Name, Ty,
    };

    #[derive(Debug, PartialEq, Eq, Clone)]
    pub(crate) enum InferenceDiagnostic {
        UnresolvedValue {
            id: ExprOrPatId,
        },
        UnresolvedType {
            id: TypeRefId,
        },
        ExpectedFunction {
            id: ExprId,
            found: Ty,
        },
        ParameterCountMismatch {
            id: ExprId,
            found: usize,
            expected: usize,
        },
        MismatchedTypes {
            id: ExprId,
            expected: Ty,
            found: Ty,
        },
        IncompatibleBranches {
            id: ExprId,
            then_ty: Ty,
            else_ty: Ty,
        },
        MissingElseBranch {
            id: ExprId,
            then_ty: Ty,
        },
        CannotApplyBinaryOp {
            id: ExprId,
            lhs: Ty,
            rhs: Ty,
        },
        CannotApplyUnaryOp {
            id: ExprId,
            ty: Ty,
        },
        InvalidLHS {
            id: ExprId,
            lhs: ExprId,
        },
        ReturnMissingExpression {
            id: ExprId,
        },
        BreakOutsideLoop {
            id: ExprId,
        },
        BreakWithValueOutsideLoop {
            id: ExprId,
        },
        AccessUnknownField {
            id: ExprId,
            receiver_ty: Ty,
            name: Name,
        },
        FieldCountMismatch {
            id: ExprId,
            found: usize,
            expected: usize,
        },
        MissingFields {
            id: ExprId,
            names: Vec<Name>,
        },
        MismatchedStructLit {
            id: ExprId,
            expected: StructKind,
            found: StructKind,
        },
        NoFields {
            id: ExprId,
            found: Ty,
        },
        NoSuchField {
            id: ExprId,
            field: usize,
        },
        LiteralOutOfRange {
            id: ExprId,
            literal_ty: IntTy,
        },
    }

    impl InferenceDiagnostic {
        pub(crate) fn add_to(
            &self,
            db: &dyn HirDatabase,
            owner: Function,
            sink: &mut DiagnosticSink,
        ) {
            let file = owner.source(db.upcast()).file_id;
            let body = owner.body_source_map(db);
            match self {
                InferenceDiagnostic::UnresolvedValue { id } => {
                    let expr = match id {
                        ExprOrPatId::ExprId(id) => body.expr_syntax(*id).map(|ptr| {
                            ptr.value
                                .either(|it| it.syntax_node_ptr(), |it| it.syntax_node_ptr())
                        }),
                        ExprOrPatId::PatId(id) => {
                            body.pat_syntax(*id).map(|ptr| ptr.value.syntax_node_ptr())
                        }
                    }
                    .unwrap();

                    sink.push(UnresolvedValue { file, expr });
                }
                InferenceDiagnostic::UnresolvedType { id } => {
                    let type_ref = body.type_ref_syntax(*id).expect("If this is not found, it must be a type ref generated by the library which should never be unresolved.");
                    sink.push(UnresolvedType { file, type_ref });
                }
                InferenceDiagnostic::ParameterCountMismatch {
                    id,
                    expected,
                    found,
                } => {
                    let expr = body
                        .expr_syntax(*id)
                        .unwrap()
                        .value
                        .either(|it| it.syntax_node_ptr(), |it| it.syntax_node_ptr());
                    sink.push(ParameterCountMismatch {
                        file,
                        expr,
                        expected: *expected,
                        found: *found,
                    })
                }
                InferenceDiagnostic::ExpectedFunction { id, found } => {
                    let expr = body
                        .expr_syntax(*id)
                        .unwrap()
                        .value
                        .either(|it| it.syntax_node_ptr(), |it| it.syntax_node_ptr());
                    sink.push(ExpectedFunction {
                        file,
                        expr,
                        found: found.clone(),
                    });
                }
                InferenceDiagnostic::MismatchedTypes {
                    id,
                    found,
                    expected,
                } => {
                    let expr = body
                        .expr_syntax(*id)
                        .unwrap()
                        .value
                        .either(|it| it.syntax_node_ptr(), |it| it.syntax_node_ptr());
                    sink.push(MismatchedType {
                        file,
                        expr,
                        found: found.clone(),
                        expected: expected.clone(),
                    });
                }
                InferenceDiagnostic::IncompatibleBranches {
                    id,
                    then_ty,
                    else_ty,
                } => {
                    let expr = body
                        .expr_syntax(*id)
                        .unwrap()
                        .value
                        .either(|it| it.syntax_node_ptr(), |it| it.syntax_node_ptr());
                    sink.push(IncompatibleBranch {
                        file,
                        if_expr: expr,
                        expected: then_ty.clone(),
                        found: else_ty.clone(),
                    });
                }
                InferenceDiagnostic::MissingElseBranch { id, then_ty } => {
                    let expr = body
                        .expr_syntax(*id)
                        .unwrap()
                        .value
                        .either(|it| it.syntax_node_ptr(), |it| it.syntax_node_ptr());
                    sink.push(MissingElseBranch {
                        file,
                        if_expr: expr,
                        found: then_ty.clone(),
                    });
                }
                InferenceDiagnostic::CannotApplyBinaryOp { id, lhs, rhs } => {
                    let expr = body
                        .expr_syntax(*id)
                        .unwrap()
                        .value
                        .either(|it| it.syntax_node_ptr(), |it| it.syntax_node_ptr());
                    sink.push(CannotApplyBinaryOp {
                        file,
                        expr,
                        lhs: lhs.clone(),
                        rhs: rhs.clone(),
                    });
                }
                InferenceDiagnostic::CannotApplyUnaryOp { id, ty } => {
                    let expr = body
                        .expr_syntax(*id)
                        .unwrap()
                        .value
                        .either(|it| it.syntax_node_ptr(), |it| it.syntax_node_ptr());
                    sink.push(CannotApplyUnaryOp {
                        file,
                        expr,
                        ty: ty.clone(),
                    });
                }
                InferenceDiagnostic::InvalidLHS { id, lhs } => {
                    let id = body
                        .expr_syntax(*id)
                        .unwrap()
                        .value
                        .either(|it| it.syntax_node_ptr(), |it| it.syntax_node_ptr());
                    let lhs = body
                        .expr_syntax(*lhs)
                        .unwrap()
                        .value
                        .either(|it| it.syntax_node_ptr(), |it| it.syntax_node_ptr());
                    sink.push(InvalidLHS {
                        file,
                        expr: id,
                        lhs,
                    });
                }
                InferenceDiagnostic::ReturnMissingExpression { id } => {
                    let id = body
                        .expr_syntax(*id)
                        .unwrap()
                        .value
                        .either(|it| it.syntax_node_ptr(), |it| it.syntax_node_ptr());
                    sink.push(ReturnMissingExpression {
                        file,
                        return_expr: id,
                    });
                }
                InferenceDiagnostic::BreakOutsideLoop { id } => {
                    let id = body
                        .expr_syntax(*id)
                        .unwrap()
                        .value
                        .either(|it| it.syntax_node_ptr(), |it| it.syntax_node_ptr());
                    sink.push(BreakOutsideLoop {
                        file,
                        break_expr: id,
                    });
                }
                InferenceDiagnostic::BreakWithValueOutsideLoop { id } => {
                    let id = body
                        .expr_syntax(*id)
                        .unwrap()
                        .value
                        .either(|it| it.syntax_node_ptr(), |it| it.syntax_node_ptr());
                    sink.push(BreakWithValueOutsideLoop {
                        file,
                        break_expr: id,
                    });
                }
                InferenceDiagnostic::AccessUnknownField {
                    id,
                    receiver_ty,
                    name,
                } => {
                    let expr = body
                        .expr_syntax(*id)
                        .unwrap()
                        .value
                        .either(|it| it.syntax_node_ptr(), |it| it.syntax_node_ptr());
                    sink.push(AccessUnknownField {
                        file,
                        expr,
                        receiver_ty: receiver_ty.clone(),
                        name: name.clone(),
                    })
                }
                InferenceDiagnostic::FieldCountMismatch {
                    id,
                    expected,
                    found,
                } => {
                    let expr = body
                        .expr_syntax(*id)
                        .unwrap()
                        .value
                        .either(|it| it.syntax_node_ptr(), |it| it.syntax_node_ptr());
                    sink.push(FieldCountMismatch {
                        file,
                        expr,
                        expected: *expected,
                        found: *found,
                    })
                }
                InferenceDiagnostic::MissingFields { id, names } => {
                    let fields = body
                        .expr_syntax(*id)
                        .unwrap()
                        .value
                        .either(|it| it.syntax_node_ptr(), |it| it.syntax_node_ptr());

                    sink.push(MissingFields {
                        file,
                        fields,
                        field_names: names.to_vec(),
                    });
                }
                InferenceDiagnostic::MismatchedStructLit {
                    id,
                    expected,
                    found,
                } => {
                    let expr = body
                        .expr_syntax(*id)
                        .unwrap()
                        .value
                        .either(|it| it.syntax_node_ptr(), |it| it.syntax_node_ptr());
                    sink.push(MismatchedStructLit {
                        file,
                        expr,
                        expected: *expected,
                        found: *found,
                    });
                }
                InferenceDiagnostic::NoFields { id, found } => {
                    let expr = body
                        .expr_syntax(*id)
                        .unwrap()
                        .value
                        .either(|it| it.syntax_node_ptr(), |it| it.syntax_node_ptr());
                    sink.push(NoFields {
                        file,
                        receiver_expr: expr,
                        found: found.clone(),
                    })
                }
                InferenceDiagnostic::NoSuchField { id, field } => {
                    let field = owner.body_source_map(db).field_syntax(*id, *field).into();
                    sink.push(NoSuchField { file, field });
                }
                InferenceDiagnostic::LiteralOutOfRange { id, literal_ty } => {
                    let literal = body
                        .expr_syntax(*id)
                        .expect("could not retrieve expr from source map")
                        .map(|expr_src| {
                            expr_src
                                .left()
                                .expect("could not retrieve expr from ExprSource")
                                .cast()
                                .expect("could not cast expression to literal")
                        });
                    sink.push(LiteralOutOfRange {
                        literal,
                        int_ty: *literal_ty,
                    })
                }
            }
        }
    }
}

use crate::{
    arena::map::ArenaMap,
    code_model::DefWithBody,
    diagnostics::DiagnosticSink,
    expr,
    expr::{Body, Expr, ExprId, Literal, Pat, PatId, Statement},
    name_resolution::Namespace,
    resolve::{Resolution, Resolver},
    ty::infer::diagnostics::InferenceDiagnostic,
    ty::infer::type_variable::TypeVariableTable,
    ty::lower::LowerDiagnostic,
    ty::op,
    ty::{Ty, TypableDef},
    type_ref::TypeRefId,
    Function, HirDatabase, Path, TypeCtor,
};
use std::mem;
use std::ops::Index;
use std::sync::Arc;

mod type_variable;

pub use type_variable::TypeVarId;

/// The result of type inference: A mapping from expressions and patterns to types.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct InferenceResult {
    type_of_expr: ArenaMap<ExprId, Ty>,
    type_of_pat: ArenaMap<PatId, Ty>,
    diagnostics: Vec<diagnostics::InferenceDiagnostic>,
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
        db: &impl HirDatabase,
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
pub fn infer_query(db: &impl HirDatabase, def: DefWithBody) -> Arc<InferenceResult> {
    let body = def.body(db);
    let resolver = def.resolver(db);
    let mut ctx = InferenceResultBuilder::new(db, body, resolver);

    match def {
        DefWithBody::Function(_) => ctx.infer_signature(),
    }

    ctx.infer_body();

    Arc::new(ctx.resolve_all())
}

/// The inference context contains all information needed during type inference.
struct InferenceResultBuilder<'a, D: HirDatabase> {
    db: &'a D,
    body: Arc<Body>,
    resolver: Resolver,

    type_of_expr: ArenaMap<ExprId, Ty>,
    type_of_pat: ArenaMap<PatId, Ty>,
    diagnostics: Vec<InferenceDiagnostic>,

    type_variables: TypeVariableTable,

    /// The return type of the function being inferred.
    return_ty: Ty,
}

impl<'a, D: HirDatabase> InferenceResultBuilder<'a, D> {
    /// Construct a new `InferenceContext` from a `Body` and a `Resolver` for that body.
    fn new(db: &'a D, body: Arc<Body>, resolver: Resolver) -> Self {
        InferenceResultBuilder {
            type_of_expr: ArenaMap::default(),
            type_of_pat: ArenaMap::default(),
            diagnostics: Vec::default(),
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
    fn resolve_type(&mut self, type_ref: &TypeRefId) -> Ty {
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

impl<'a, D: HirDatabase> InferenceResultBuilder<'a, D> {
    /// Collect all the parameter patterns from the body. After calling this method the `return_ty`
    /// will have a valid value, also all parameters are added inferred.
    fn infer_signature(&mut self) {
        let body = Arc::clone(&self.body); // avoid borrow checker problem

        // Iterate over all the parameters and associated types of the body and infer the types of
        // the parameters.
        for (pat, type_ref) in body.params().iter() {
            let ty = self.resolve_type(type_ref);
            self.infer_pat(*pat, ty);
        }

        // Resolve the return type
        self.return_ty = self.resolve_type(&body.ret_type())
    }

    /// Record the type of the specified pattern and all sub-patterns.
    fn infer_pat(&mut self, pat: PatId, ty: Ty) {
        let body = Arc::clone(&self.body); // avoid borrow checker problem
        match &body[pat] {
            Pat::Bind { .. } => {
                self.set_pat_type(pat, ty);
            }
            _ => {}
        }
    }

    /// Infer the types of all the expressions and sub-expressions in the body.
    fn infer_body(&mut self) {
        self.infer_expr(
            self.body.body_expr(),
            &Expectation::has_type(self.return_ty.clone()),
        );
    }

    /// Infer the type of the given expression. Returns the type of the expression.
    fn infer_expr(&mut self, tgt_expr: ExprId, expected: &Expectation) -> Ty {
        let body = Arc::clone(&self.body); // avoid borrow checker problem
        let mut ty = match &body[tgt_expr] {
            Expr::Missing => Ty::Unknown,
            Expr::Path(p) => {
                // FIXME this could be more efficient...
                let resolver = expr::resolver_for_expr(self.body.clone(), self.db, tgt_expr);
                self.infer_path_expr(&resolver, p, tgt_expr.into())
                    .unwrap_or(Ty::Unknown)
            }
            Expr::BinaryOp { lhs, rhs, op } => match op {
                Some(op) => {
                    let lhs_ty = self.infer_expr(*lhs, &Expectation::none());
                    let rhs_expected = op::binary_op_rhs_expectation(*op, lhs_ty.clone());
                    let rhs_ty = self.infer_expr(*rhs, &Expectation::has_type(rhs_expected));
                    op::binary_op_return_ty(*op, rhs_ty)
                }
                _ => Ty::Unknown,
            },
            Expr::Block { statements, tail } => self.infer_block(statements, *tail, expected),
            Expr::Call { callee: call, args } => self.infer_call(&tgt_expr, call, args, expected),
            Expr::Literal(lit) => match lit {
                Literal::String(_) => Ty::Unknown,
                Literal::Bool(_) => Ty::Unknown,
                Literal::Int(_) => Ty::simple(TypeCtor::Int),
                Literal::Float(_) => Ty::simple(TypeCtor::Float),
            },
            _ => Ty::Unknown,
            //            Expr::UnaryOp { expr: _, op: _ } => {}
            //            Expr::Block { statements: _, tail: _ } => {}
        };

        if expected.ty != Ty::Unknown && ty != Ty::Unknown && ty != expected.ty {
            self.diagnostics.push(InferenceDiagnostic::MismatchedTypes {
                expected: expected.ty.clone(),
                found: ty.clone(),
                id: tgt_expr,
            });
            ty = expected.ty.clone();
        }

        self.set_expr_type(tgt_expr, ty.clone());
        ty
    }

    /// Inferences the type of a call expression.
    fn infer_call(
        &mut self,
        tgt_expr: &ExprId,
        callee: &ExprId,
        args: &Vec<ExprId>,
        _expected: &Expectation,
    ) -> Ty {
        let callee_ty = self.infer_expr(*callee, &Expectation::none());
        let (param_tys, ret_ty) = match callee_ty.callable_sig(self.db) {
            Some(sig) => (sig.params().to_vec(), sig.ret().clone()),
            None => {
                self.diagnostics
                    .push(InferenceDiagnostic::ExpectedFunction {
                        id: *callee,
                        found: callee_ty,
                    });
                (Vec::new(), Ty::Unknown)
            }
        };
        self.check_call_arguments(tgt_expr, args, &param_tys);
        ret_ty
    }

    /// Checks whether the specified passed arguments match the parameters of a callable definition.
    fn check_call_arguments(&mut self, tgt_expr: &ExprId, args: &[ExprId], param_tys: &[Ty]) {
        if args.len() != param_tys.len() {
            self.diagnostics
                .push(InferenceDiagnostic::ParameterCountMismatch {
                    id: *tgt_expr,
                    found: args.len(),
                    expected: param_tys.len(),
                })
        }
        for (&arg, param_ty) in args.iter().zip(param_tys.iter()) {
            self.infer_expr(arg, &Expectation::has_type(param_ty.clone()));
        }
    }

    fn infer_path_expr(&mut self, resolver: &Resolver, path: &Path, id: ExprOrPatId) -> Option<Ty> {
        let resolution = match resolver
            .resolve_path_without_assoc_items(self.db, path)
            .take_values()
        {
            Some(resolution) => resolution,
            None => {
                self.diagnostics
                    .push(InferenceDiagnostic::UnresolvedValue { id });
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
                Some(ty)
            }
        }
    }

    fn resolve_all(mut self) -> InferenceResult {
        // FIXME resolve obligations as well (use Guidance if necessary)
        //let mut tv_stack = Vec::new();
        let mut expr_types = mem::replace(&mut self.type_of_expr, ArenaMap::default());
        for (expr, ty) in expr_types.iter_mut() {
            //let resolved = self.resolve_ty_completely(&mut tv_stack, mem::replace(ty, Ty::Unknown));
            if *ty == Ty::Unknown {
                self.report_expr_inference_failure(expr);
            }
            //*ty = resolved;
        }
        let mut pat_types = mem::replace(&mut self.type_of_pat, ArenaMap::default());
        for (pat, ty) in pat_types.iter_mut() {
            //let resolved = self.resolve_ty_completely(&mut tv_stack, mem::replace(ty, Ty::Unknown));
            if *ty == Ty::Unknown {
                self.report_pat_inference_failure(pat);
            }
            //*ty = resolved;
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

    fn infer_block(
        &mut self,
        statements: &[Statement],
        tail: Option<ExprId>,
        expected: &Expectation,
    ) -> Ty {
        for stmt in statements {
            match stmt {
                Statement::Let {
                    pat,
                    type_ref,
                    initializer,
                } => {
                    let decl_ty = type_ref
                        .as_ref()
                        .map(|tr| self.resolve_type(tr))
                        .unwrap_or(Ty::Unknown);
                    //let decl_ty = self.insert_type_vars(decl_ty);
                    let ty = if let Some(expr) = initializer {
                        self.infer_expr(*expr, &Expectation::has_type(decl_ty))
                    } else {
                        decl_ty
                    };

                    self.infer_pat(*pat, ty);
                }
                Statement::Expr(expr) => {
                    self.infer_expr(*expr, &Expectation::none());
                }
            }
        }
        if let Some(expr) = tail {
            self.infer_expr(expr, expected)
        } else {
            Ty::Empty
        }
    }

    pub fn report_pat_inference_failure(&mut self, _pat: PatId) {
        //        self.diagnostics.push(InferenceDiagnostic::PatInferenceFailed {
        //            pat
        //        });
    }

    pub fn report_expr_inference_failure(&mut self, _expr: ExprId) {
        //        self.diagnostics.push(InferenceDiagnostic::ExprInferenceFailed {
        //            expr
        //        });
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
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub(super) enum ExprOrPatId {
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
        CannotApplyBinaryOp, ExpectedFunction, MismatchedType, ParameterCountMismatch,
    };
    use crate::{
        code_model::src::HasSource,
        diagnostics::{DiagnosticSink, UnresolvedType, UnresolvedValue},
        ty::infer::ExprOrPatId,
        type_ref::TypeRefId,
        ExprId, Function, HirDatabase, Ty,
    };

    #[derive(Debug, PartialEq, Eq, Clone)]
    pub(super) enum InferenceDiagnostic {
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
        CannotApplyBinaryOp {
            id: ExprId,
            lhs: Ty,
            rhs: Ty,
        },
    }

    impl InferenceDiagnostic {
        pub(super) fn add_to(
            &self,
            db: &impl HirDatabase,
            owner: Function,
            sink: &mut DiagnosticSink,
        ) {
            match self {
                InferenceDiagnostic::UnresolvedValue { id } => {
                    let file = owner.source(db).file_id;
                    let body = owner.body_source_map(db);
                    let expr = match id {
                        ExprOrPatId::ExprId(id) => body.expr_syntax(*id),
                        ExprOrPatId::PatId(id) => {
                            body.pat_syntax(*id).map(|ptr| ptr.syntax_node_ptr())
                        }
                    }
                    .unwrap();

                    sink.push(UnresolvedValue { file, expr });
                }
                InferenceDiagnostic::UnresolvedType { id } => {
                    let file = owner.source(db).file_id;
                    let body = owner.body_source_map(db);
                    let type_ref = body.type_ref_syntax(*id).expect("If this is not found, it must be a type ref generated by the library which should never be unresolved.");
                    sink.push(UnresolvedType { file, type_ref });
                }
                InferenceDiagnostic::ParameterCountMismatch {
                    id,
                    expected,
                    found,
                } => {
                    let file = owner.source(db).file_id;
                    let body = owner.body_source_map(db);
                    let expr = body.expr_syntax(*id).unwrap();
                    sink.push(ParameterCountMismatch {
                        file,
                        expr,
                        expected: *expected,
                        found: *found,
                    })
                }
                InferenceDiagnostic::ExpectedFunction { id, found } => {
                    let file = owner.source(db).file_id;
                    let body = owner.body_source_map(db);
                    let expr = body.expr_syntax(*id).unwrap();
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
                    let file = owner.source(db).file_id;
                    let body = owner.body_source_map(db);
                    let expr = body.expr_syntax(*id).unwrap();
                    sink.push(MismatchedType {
                        file,
                        expr,
                        found: found.clone(),
                        expected: expected.clone(),
                    });
                }
                InferenceDiagnostic::CannotApplyBinaryOp { id, lhs, rhs } => {
                    let file = owner.source(db).file_id;
                    let body = owner.body_source_map(db);
                    let expr = body.expr_syntax(*id).unwrap();
                    sink.push(CannotApplyBinaryOp {
                        file,
                        expr,
                        lhs: lhs.clone(),
                        rhs: rhs.clone(),
                    });
                }
            }
        }
    }
}

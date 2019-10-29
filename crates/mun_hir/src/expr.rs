use crate::{
    arena::map::ArenaMap,
    arena::{Arena, RawId},
    code_model::DefWithBody,
    FileId, HirDatabase, Name, Path,
};

//pub use mun_syntax::ast::PrefixOp as UnaryOp;
use crate::code_model::src::{HasSource, Source};
use crate::name::AsName;
use crate::type_ref::{TypeRef, TypeRefBuilder, TypeRefId, TypeRefMap, TypeRefSourceMap};
pub use mun_syntax::ast::PrefixOp as UnaryOp;
use mun_syntax::ast::{ArgListOwner, BinOp, NameOwner, TypeAscriptionOwner};
use mun_syntax::{ast, AstNode, AstPtr, T};
use rustc_hash::FxHashMap;
use std::ops::Index;
use std::sync::Arc;

pub use self::scope::ExprScopes;
use crate::resolve::Resolver;
use std::mem;

pub(crate) mod scope;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExprId(RawId);
impl_arena_id!(ExprId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PatId(RawId);
impl_arena_id!(PatId);

/// The body of an item (function, const etc.).
#[derive(Debug, Eq, PartialEq)]
pub struct Body {
    owner: DefWithBody,
    exprs: Arena<ExprId, Expr>,
    pats: Arena<PatId, Pat>,
    type_refs: TypeRefMap,
    /// The patterns for the function's parameters. While the parameter types are part of the
    /// function signature, the patterns are not (they don't change the external type of the
    /// function).
    ///
    /// If this `Body` is for the body of a constant, this will just be empty.
    params: Vec<(PatId, TypeRefId)>,
    /// The `ExprId` of the actual body expression.
    body_expr: ExprId,
    ret_type: TypeRefId,
}

impl Body {
    pub fn params(&self) -> &[(PatId, TypeRefId)] {
        &self.params
    }

    pub fn body_expr(&self) -> ExprId {
        self.body_expr
    }

    pub fn owner(&self) -> DefWithBody {
        self.owner
    }

    pub fn exprs(&self) -> impl Iterator<Item = (ExprId, &Expr)> {
        self.exprs.iter()
    }

    pub fn pats(&self) -> impl Iterator<Item = (PatId, &Pat)> {
        self.pats.iter()
    }

    pub fn type_refs(&self) -> &TypeRefMap {
        &self.type_refs
    }

    pub fn ret_type(&self) -> TypeRefId {
        self.ret_type
    }
}

impl Index<ExprId> for Body {
    type Output = Expr;

    fn index(&self, expr: ExprId) -> &Expr {
        &self.exprs[expr]
    }
}

impl Index<PatId> for Body {
    type Output = Pat;

    fn index(&self, pat: PatId) -> &Pat {
        &self.pats[pat]
    }
}

impl Index<TypeRefId> for Body {
    type Output = TypeRef;

    fn index(&self, type_ref: TypeRefId) -> &TypeRef {
        &self.type_refs[type_ref]
    }
}

type ExprPtr = AstPtr<ast::Expr>; //Either<AstPtr<ast::Pat>, AstPtr<ast::SelfParam>>;
type ExprSource = Source<ExprPtr>;

type PatPtr = AstPtr<ast::Pat>; //Either<AstPtr<ast::Pat>, AstPtr<ast::SelfParam>>;
type PatSource = Source<PatPtr>;

/// An item body together with the mapping from syntax nodes to HIR expression Ids. This is needed
/// to go from e.g. a position in a file to the HIR expression containing it; but for type
/// inference etc., we want to operate on a structure that is agnostic to the action positions of
/// expressions in the file, so that we don't recompute types whenever some whitespace is typed.
#[derive(Default, Debug, Eq, PartialEq)]
pub struct BodySourceMap {
    expr_map: FxHashMap<ExprPtr, ExprId>,
    expr_map_back: ArenaMap<ExprId, ExprSource>,
    pat_map: FxHashMap<PatPtr, PatId>,
    pat_map_back: ArenaMap<PatId, PatSource>,
    type_refs: TypeRefSourceMap,
}

impl BodySourceMap {
    pub(crate) fn expr_syntax(&self, expr: ExprId) -> Option<ExprSource> {
        self.expr_map_back.get(expr).cloned()
    }

    pub fn type_ref_syntax(&self, type_ref: TypeRefId) -> Option<AstPtr<ast::TypeRef>> {
        self.type_refs.type_ref_syntax(type_ref)
    }

    pub(crate) fn syntax_expr(&self, ptr: ExprPtr) -> Option<ExprId> {
        self.expr_map.get(&ptr).cloned()
    }

    pub(crate) fn node_expr(&self, node: &ast::Expr) -> Option<ExprId> {
        self.expr_map.get(&AstPtr::new(node)).cloned()
    }

    pub(crate) fn pat_syntax(&self, pat: PatId) -> Option<PatSource> {
        self.pat_map_back.get(pat).cloned()
    }

    pub(crate) fn node_pat(&self, node: &ast::Pat) -> Option<PatId> {
        self.pat_map.get(&AstPtr::new(node)).cloned()
    }

    pub fn type_refs(&self) -> &TypeRefSourceMap {
        &self.type_refs
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Statement {
    Let {
        pat: PatId,
        type_ref: Option<TypeRefId>,
        initializer: Option<ExprId>,
    },
    Expr(ExprId),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    String(String),
    Bool(bool),
    Int(i64),
    Float(f64),
}

impl Eq for Literal {}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Expr {
    /// Used if the syntax tree does not have a required expression piece
    Missing,
    Call {
        callee: ExprId,
        args: Vec<ExprId>,
    },
    Path(Path),
    If {
        condition: ExprId,
        then_branch: ExprId,
        else_branch: Option<ExprId>,
    },
    UnaryOp {
        expr: ExprId,
        op: UnaryOp,
    },
    BinaryOp {
        lhs: ExprId,
        rhs: ExprId,
        op: Option<BinaryOp>,
    },
    Block {
        statements: Vec<Statement>,
        tail: Option<ExprId>,
    },
    Literal(Literal),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    LogicOp(LogicOp),
    ArithOp(ArithOp),
    CmpOp(CmpOp),
    Assignment, // { op: Option<ArithOp> }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum LogicOp {
    And,
    Or,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CmpOp {
    Eq { negated: bool },
    Ord { ordering: Ordering, strict: bool },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Ordering {
    Less,
    Greater,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ArithOp {
    Add,
    Multiply,
    Subtract,
    Divide,
    Remainder,
    Power,
}

impl Expr {
    pub fn walk_child_exprs(&self, mut f: impl FnMut(ExprId)) {
        match self {
            Expr::Missing => {}
            Expr::Path(_) => {}
            Expr::Block { statements, tail } => {
                for stmt in statements {
                    match stmt {
                        Statement::Let { initializer, .. } => {
                            if let Some(expr) = initializer {
                                f(*expr);
                            }
                        }
                        Statement::Expr(e) => f(*e),
                    }
                }
                if let Some(expr) = tail {
                    f(*expr);
                }
            }
            Expr::Call { callee, args } => {
                f(*callee);
                for arg in args {
                    f(*arg);
                }
            }
            Expr::BinaryOp { lhs, rhs, .. } => {
                f(*lhs);
                f(*rhs);
            }
            Expr::UnaryOp { expr, .. } => {
                f(*expr);
            }
            Expr::Literal(_) => {}
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                f(*condition);
                f(*then_branch);
                if let Some(else_expr) = else_branch {
                    f(*else_expr);
                }
            }
        }
    }
}

/// Similar to `ast::PatKind`
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Pat {
    Missing,             // Indicates an error
    Wild,                // `_`
    Path(Path),          // E.g. `foo::bar`
    Bind { name: Name }, // E.g. `a`
}

impl Pat {
    pub fn walk_child_pats(&self, mut _f: impl FnMut(PatId)) {
        unreachable!()
    }
}

// Queries

pub(crate) struct ExprCollector<DB> {
    db: DB,
    owner: DefWithBody,
    exprs: Arena<ExprId, Expr>,
    pats: Arena<PatId, Pat>,
    source_map: BodySourceMap,
    params: Vec<(PatId, TypeRefId)>,
    body_expr: Option<ExprId>,
    ret_type: Option<TypeRefId>,
    type_ref_builder: TypeRefBuilder,
    current_file_id: FileId,
}

impl<'a, DB> ExprCollector<&'a DB>
where
    DB: HirDatabase,
{
    pub fn new(owner: DefWithBody, file_id: FileId, db: &'a DB) -> Self {
        ExprCollector {
            owner,
            db,
            exprs: Arena::default(),
            pats: Arena::default(),
            source_map: BodySourceMap::default(),
            params: Vec::new(),
            body_expr: None,
            ret_type: None,
            type_ref_builder: TypeRefBuilder::default(),
            current_file_id: file_id,
        }
    }

    fn alloc_pat(&mut self, pat: Pat, ptr: PatPtr) -> PatId {
        let id = self.pats.alloc(pat);
        self.source_map.pat_map.insert(ptr, id);
        self.source_map.pat_map_back.insert(
            id,
            Source {
                file_id: self.current_file_id,
                ast: ptr,
            },
        );
        id
    }

    fn alloc_expr(&mut self, expr: Expr, ptr: ExprPtr) -> ExprId {
        let id = self.exprs.alloc(expr);
        self.source_map.expr_map.insert(ptr, id);
        self.source_map.expr_map_back.insert(
            id,
            Source {
                file_id: self.current_file_id,
                ast: ptr,
            },
        );
        id
    }

    fn missing_expr(&mut self) -> ExprId {
        self.exprs.alloc(Expr::Missing)
    }

    fn collect_fn_body(&mut self, node: &ast::FunctionDef) {
        if let Some(param_list) = node.param_list() {
            for param in param_list.params() {
                let pat = if let Some(pat) = param.pat() {
                    pat
                } else {
                    continue;
                };
                let param_pat = self.collect_pat(pat);
                let param_type = self
                    .type_ref_builder
                    .alloc_from_node_opt(param.ascribed_type().as_ref());
                self.params.push((param_pat, param_type));
            }
        }

        let body = self.collect_block_opt(node.body());
        self.body_expr = Some(body);

        let ret_type = if let Some(type_ref) = node.ret_type().and_then(|rt| rt.type_ref()) {
            self.type_ref_builder.alloc_from_node(&type_ref)
        } else {
            self.type_ref_builder.unit()
        };
        self.ret_type = Some(ret_type);
    }

    fn collect_block_opt(&mut self, block: Option<ast::BlockExpr>) -> ExprId {
        if let Some(block) = block {
            self.collect_block(block)
        } else {
            self.exprs.alloc(Expr::Missing)
        }
    }

    fn collect_block(&mut self, block: ast::BlockExpr) -> ExprId {
        let syntax_node_ptr = AstPtr::new(&block.clone().into());
        let statements = block
            .statements()
            .map(|s| match s.kind() {
                ast::StmtKind::LetStmt(stmt) => {
                    let pat = self.collect_pat_opt(stmt.pat());
                    let type_ref = stmt
                        .ascribed_type()
                        .map(|t| self.type_ref_builder.alloc_from_node(&t));
                    let initializer = stmt.initializer().map(|e| self.collect_expr(e));
                    Statement::Let {
                        pat,
                        type_ref,
                        initializer,
                    }
                }
                ast::StmtKind::ExprStmt(stmt) => {
                    Statement::Expr(self.collect_expr_opt(stmt.expr()))
                }
            })
            .collect();
        let tail = block.expr().map(|e| self.collect_expr(e));
        self.alloc_expr(Expr::Block { statements, tail }, syntax_node_ptr)
    }

    fn collect_pat_opt(&mut self, pat: Option<ast::Pat>) -> PatId {
        if let Some(pat) = pat {
            self.collect_pat(pat)
        } else {
            self.pats.alloc(Pat::Missing)
        }
    }

    fn collect_expr_opt(&mut self, expr: Option<ast::Expr>) -> ExprId {
        if let Some(expr) = expr {
            self.collect_expr(expr)
        } else {
            self.exprs.alloc(Expr::Missing)
        }
    }

    fn collect_expr(&mut self, expr: ast::Expr) -> ExprId {
        let syntax_ptr = AstPtr::new(&expr.clone());
        match expr.kind() {
            ast::ExprKind::BlockExpr(b) => self.collect_block(b),
            ast::ExprKind::Literal(e) => {
                let lit = match e.kind() {
                    ast::LiteralKind::Bool => Literal::Bool(e.syntax().kind() == T![true]),
                    ast::LiteralKind::IntNumber => {
                        Literal::Int(e.syntax().text().to_string().parse().unwrap())
                    }
                    ast::LiteralKind::FloatNumber => {
                        Literal::Float(e.syntax().text().to_string().parse().unwrap())
                    }
                    ast::LiteralKind::String => Literal::String(Default::default()),
                };
                self.alloc_expr(Expr::Literal(lit), syntax_ptr)
            }
            ast::ExprKind::PrefixExpr(e) => {
                let expr = self.collect_expr_opt(e.expr());
                if let Some(op) = e.op_kind() {
                    self.alloc_expr(Expr::UnaryOp { expr, op }, syntax_ptr)
                } else {
                    self.alloc_expr(Expr::Missing, syntax_ptr)
                }
            }
            ast::ExprKind::BinExpr(e) => {
                let op = e.op_kind();
                if let Some(op) = op {
                    match op {
                        op @ BinOp::Add
                        | op @ BinOp::Subtract
                        | op @ BinOp::Divide
                        | op @ BinOp::Multiply
                        | op @ BinOp::Equals
                        | op @ BinOp::NotEquals
                        | op @ BinOp::Less
                        | op @ BinOp::LessEqual
                        | op @ BinOp::Greater
                        | op @ BinOp::GreatEqual
                        //| op @ BinOp::Remainder
                        //| op @ BinOp::Power
                        => {
                            let op = match op {
                                BinOp::Add => BinaryOp::ArithOp(ArithOp::Add),
                                BinOp::Subtract => BinaryOp::ArithOp(ArithOp::Subtract),
                                BinOp::Divide => BinaryOp::ArithOp(ArithOp::Divide),
                                BinOp::Multiply => BinaryOp::ArithOp(ArithOp::Multiply),
                                BinOp::Equals => BinaryOp::CmpOp(CmpOp::Eq { negated: false }),
                                BinOp::NotEquals => BinaryOp::CmpOp(CmpOp::Eq { negated: true }),
                                BinOp::Less => BinaryOp::CmpOp(CmpOp::Ord { ordering: Ordering::Less, strict: true } ),
                                BinOp::LessEqual => BinaryOp::CmpOp(CmpOp::Ord { ordering: Ordering::Less, strict: false } ),
                                BinOp::Greater => BinaryOp::CmpOp(CmpOp::Ord { ordering: Ordering::Greater, strict: true } ),
                                BinOp::GreatEqual => BinaryOp::CmpOp(CmpOp::Ord { ordering: Ordering::Greater, strict: false } ),
                                //BinOp::Remainder => BinaryOp::ArithOp(ArithOp::Remainder),
                                //BinOp::Power => BinaryOp::ArithOp(ArithOp::Power),
                                _ => unreachable!(),
                            };
                            let lhs = self.collect_expr_opt(e.lhs());
                            let rhs = self.collect_expr_opt(e.rhs());
                            self.alloc_expr(
                                Expr::BinaryOp {
                                    lhs,
                                    rhs,
                                    op: Some(op),
                                },
                                syntax_ptr,
                            )
                        }
                        BinOp::Assign => {
                            let lhs = self.collect_expr_opt(e.lhs());
                            let rhs = self.collect_expr_opt(e.rhs());
                            self.alloc_expr(
                                Expr::BinaryOp {
                                    lhs,
                                    rhs,
                                    op: Some(BinaryOp::Assignment),
                                },
                                syntax_ptr,
                            )
                        }
                        op @ BinOp::AddAssign
                        | op @ BinOp::SubtractAssign
                        | op @ BinOp::DivideAssign
                        | op @ BinOp::MultiplyAssign
                        //| op @ BinOp::RemainderAssign
                        //| op @ BinOp::PowerAssign
                        => {
                            let op = match op {
                                BinOp::AddAssign => BinaryOp::ArithOp(ArithOp::Add),
                                BinOp::SubtractAssign => BinaryOp::ArithOp(ArithOp::Subtract),
                                BinOp::DivideAssign => BinaryOp::ArithOp(ArithOp::Divide),
                                BinOp::MultiplyAssign => BinaryOp::ArithOp(ArithOp::Multiply),
                                //BinOp::RemainderAssign => BinaryOp::ArithOp(ArithOp::Remainder),
                                //BinOp::PowerAssign => BinaryOp::ArithOp(ArithOp::Power),
                                _ => unreachable!(),
                            };
                            let lhs = self.collect_expr_opt(e.lhs());
                            let lhs_rhs = self.collect_expr_opt(e.lhs());
                            let rhs = self.collect_expr_opt(e.rhs());
                            let update_expr = self.alloc_expr(
                                Expr::BinaryOp {
                                    lhs: lhs_rhs,
                                    rhs,
                                    op: Some(op),
                                },
                                syntax_ptr,
                            );
                            self.alloc_expr(
                                Expr::BinaryOp {
                                    lhs,
                                    rhs: update_expr,
                                    op: Some(op),
                                },
                                syntax_ptr,
                            )
                        }
                    }
                } else {
                    let lhs = self.collect_expr_opt(e.lhs());
                    let rhs = self.collect_expr_opt(e.rhs());
                    self.alloc_expr(Expr::BinaryOp { lhs, rhs, op: None }, syntax_ptr)
                }
            }
            ast::ExprKind::PathExpr(e) => {
                let path = e
                    .path()
                    .and_then(Path::from_ast)
                    .map(Expr::Path)
                    .unwrap_or(Expr::Missing);
                self.alloc_expr(path, syntax_ptr)
            }
            ast::ExprKind::IfExpr(e) => {
                let then_branch = self.collect_block_opt(e.then_branch());

                let else_branch = e.else_branch().map(|b| match b {
                    ast::ElseBranch::Block(it) => self.collect_block(it),
                    ast::ElseBranch::IfExpr(elif) => {
                        let expr = ast::Expr::cast(elif.syntax().clone()).unwrap();
                        self.collect_expr(expr)
                    }
                });

                let condition = match e.condition() {
                    None => self.missing_expr(),
                    Some(condition) => match condition.pat() {
                        None => self.collect_expr_opt(condition.expr()),
                        _ => unreachable!("patterns in conditions are not yet supported"),
                    },
                };

                self.alloc_expr(
                    Expr::If {
                        condition,
                        then_branch,
                        else_branch,
                    },
                    syntax_ptr,
                )
            }
            ast::ExprKind::ParenExpr(e) => {
                let inner = self.collect_expr_opt(e.expr());
                // make the paren expr point to the inner expression as well
                self.source_map.expr_map.insert(syntax_ptr, inner);
                inner
            }
            ast::ExprKind::CallExpr(e) => {
                let callee = self.collect_expr_opt(e.expr());
                let args = if let Some(arg_list) = e.arg_list() {
                    arg_list.args().map(|e| self.collect_expr(e)).collect()
                } else {
                    Vec::new()
                };
                self.alloc_expr(Expr::Call { callee, args }, syntax_ptr)
            }
        }
    }

    fn collect_pat(&mut self, pat: ast::Pat) -> PatId {
        let pattern = match pat.kind() {
            ast::PatKind::BindPat(bp) => {
                let name = bp
                    .name()
                    .map(|nr| nr.as_name())
                    .unwrap_or_else(Name::missing);
                Pat::Bind { name }
            }
            ast::PatKind::PlaceholderPat(_) => Pat::Wild,
        };
        let ptr = AstPtr::new(&pat);
        self.alloc_pat(pattern, ptr)
    }

    fn finish(mut self) -> (Body, BodySourceMap) {
        let (type_refs, type_ref_source_map) = self.type_ref_builder.finish();
        let body = Body {
            owner: self.owner,
            exprs: self.exprs,
            pats: self.pats,
            params: self.params,
            body_expr: self.body_expr.expect("A body should have been collected"),
            type_refs,
            ret_type: self
                .ret_type
                .expect("A body should have return type collected"),
        };
        mem::replace(&mut self.source_map.type_refs, type_ref_source_map);
        (body, self.source_map)
    }
}

pub(crate) fn body_with_source_map_query(
    db: &impl HirDatabase,
    def: DefWithBody,
) -> (Arc<Body>, Arc<BodySourceMap>) {
    let mut collector;

    match def {
        DefWithBody::Function(ref f) => {
            let src = f.source(db);
            collector = ExprCollector::new(def, src.file_id, db);
            collector.collect_fn_body(&src.ast)
        }
    }

    let (body, source_map) = collector.finish();
    (Arc::new(body), Arc::new(source_map))
}

pub(crate) fn body_hir_query(db: &impl HirDatabase, def: DefWithBody) -> Arc<Body> {
    db.body_with_source_map(def).0
}

// needs arbitrary_self_types to be a method... or maybe move to the def?
pub fn resolver_for_expr(body: Arc<Body>, db: &impl HirDatabase, expr_id: ExprId) -> Resolver {
    let scopes = db.expr_scopes(body.owner);
    resolver_for_scope(body, db, scopes.scope_for(expr_id))
}

pub(crate) fn resolver_for_scope(
    body: Arc<Body>,
    db: &impl HirDatabase,
    scope_id: Option<scope::ScopeId>,
) -> Resolver {
    let mut r = body.owner.resolver(db);
    let scopes = db.expr_scopes(body.owner);
    let scope_chain = scopes.scope_chain(scope_id).collect::<Vec<_>>();
    for scope in scope_chain.into_iter().rev() {
        r = r.push_expr_scope(Arc::clone(&scopes), scope);
    }
    r
}

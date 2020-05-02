use crate::{
    arena::map::ArenaMap,
    arena::{Arena, RawId},
    code_model::DefWithBody,
    FileId, HirDatabase, Name, Path,
};

//pub use mun_syntax::ast::PrefixOp as UnaryOp;
use crate::code_model::src::HasSource;
use crate::name::AsName;
use crate::type_ref::{TypeRef, TypeRefBuilder, TypeRefId, TypeRefMap, TypeRefSourceMap};
use either::Either;
pub use mun_syntax::ast::PrefixOp as UnaryOp;
use mun_syntax::ast::{ArgListOwner, BinOp, LoopBodyOwner, NameOwner, TypeAscriptionOwner};
use mun_syntax::{ast, AstNode, AstPtr, SmolStr, T};
use rustc_hash::FxHashMap;
use std::ops::Index;
use std::sync::Arc;

pub use self::scope::ExprScopes;
use crate::builtin_type::{BuiltinFloat, BuiltinInt};
use crate::diagnostics::DiagnosticSink;
use crate::in_file::InFile;
use crate::resolve::Resolver;
use std::borrow::Cow;
use std::mem;
use std::str::FromStr;

pub(crate) mod scope;
pub(crate) mod validator;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExprId(RawId);
impl_arena_id!(ExprId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PatId(RawId);
impl_arena_id!(PatId);

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ExprDiagnostic {
    LiteralError { expr: ExprId, err: LiteralError },
}

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

    /// Diagnostics encountered when parsing the ast expressions
    diagnostics: Vec<ExprDiagnostic>,
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

    /// Adds all the `InferenceDiagnostic`s of the result to the `DiagnosticSink`.
    pub(crate) fn add_diagnostics(
        &self,
        db: &impl HirDatabase,
        owner: DefWithBody,
        sink: &mut DiagnosticSink,
    ) {
        self.diagnostics
            .iter()
            .for_each(|it| it.add_to(db, owner, sink))
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

type ExprPtr = Either<AstPtr<ast::Expr>, AstPtr<ast::RecordField>>;
type ExprSource = InFile<ExprPtr>;

type PatPtr = AstPtr<ast::Pat>; //Either<AstPtr<ast::Pat>, AstPtr<ast::SelfParam>>;
type PatSource = InFile<PatPtr>;

type RecordPtr = AstPtr<ast::RecordField>;

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
    field_map: FxHashMap<(ExprId, usize), RecordPtr>,
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
        self.expr_map.get(&Either::Left(AstPtr::new(node))).cloned()
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

    pub fn field_syntax(&self, expr: ExprId, field: usize) -> RecordPtr {
        self.field_map[&(expr, field)]
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RecordLitField {
    pub name: Name,
    pub expr: ExprId,
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
    Int(LiteralInt),
    Float(LiteralFloat),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum LiteralError {
    /// We cannot parse the integer because its too large to fit in memory
    IntTooLarge,

    /// A lexer error occurred. This might happen if the literal is malformed (e.g. 0b01012)
    LexerError,

    /// Encountered an unknown suffix
    InvalidIntSuffix(SmolStr),

    /// Encountered an unknown suffix
    InvalidFloatSuffix(SmolStr),

    /// Trying to add floating point suffix to a literal that is not a floating point number
    NonDecimalFloat(u32),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LiteralInt {
    pub kind: LiteralIntKind,
    pub value: u128,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum LiteralIntKind {
    Suffixed(BuiltinInt),
    Unsuffixed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LiteralFloat {
    pub kind: LiteralFloatKind,
    pub value: f64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum LiteralFloatKind {
    Suffixed(BuiltinFloat),
    Unsuffixed,
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
    Return {
        expr: Option<ExprId>,
    },
    Break {
        expr: Option<ExprId>,
    },
    Loop {
        body: ExprId,
    },
    While {
        condition: ExprId,
        body: ExprId,
    },
    RecordLit {
        type_id: TypeRefId,
        fields: Vec<RecordLitField>,
        spread: Option<ExprId>,
    },
    Field {
        expr: ExprId,
        name: Name,
    },
    Literal(Literal),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    LogicOp(LogicOp),
    ArithOp(ArithOp),
    CmpOp(CmpOp),
    Assignment { op: Option<ArithOp> },
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
    LeftShift,
    RightShift,
    BitAnd,
    BitOr,
    BitXor,
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
            Expr::Field { expr, .. } | Expr::UnaryOp { expr, .. } => {
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
            Expr::Return { expr } => {
                if let Some(expr) = expr {
                    f(*expr);
                }
            }
            Expr::Break { expr } => {
                if let Some(expr) = expr {
                    f(*expr);
                }
            }
            Expr::Loop { body } => {
                f(*body);
            }
            Expr::While { condition, body } => {
                f(*condition);
                f(*body);
            }
            Expr::RecordLit { fields, spread, .. } => {
                for field in fields {
                    f(field.expr);
                }
                if let Some(expr) = spread {
                    f(*expr);
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
    pub fn walk_child_pats(&self, mut _f: impl FnMut(PatId)) {}
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
    diagnostics: Vec<ExprDiagnostic>,
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
            diagnostics: Vec::new(),
        }
    }

    fn alloc_pat(&mut self, pat: Pat, ptr: PatPtr) -> PatId {
        let id = self.pats.alloc(pat);
        self.source_map.pat_map.insert(ptr, id);
        self.source_map
            .pat_map_back
            .insert(id, InFile::new(self.current_file_id, ptr));
        id
    }

    fn alloc_expr(&mut self, expr: Expr, ptr: AstPtr<ast::Expr>) -> ExprId {
        let ptr = Either::Left(ptr);
        let id = self.exprs.alloc(expr);
        self.source_map.expr_map.insert(ptr, id);
        self.source_map
            .expr_map_back
            .insert(id, InFile::new(self.current_file_id, ptr));
        id
    }

    fn alloc_expr_field_shorthand(&mut self, expr: Expr, ptr: RecordPtr) -> ExprId {
        let ptr = Either::Right(ptr);
        let id = self.exprs.alloc(expr);
        self.source_map.expr_map.insert(ptr, id);
        self.source_map
            .expr_map_back
            .insert(id, InFile::new(self.current_file_id, ptr));
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
            ast::ExprKind::LoopExpr(expr) => self.collect_loop(expr),
            ast::ExprKind::WhileExpr(expr) => self.collect_while(expr),
            ast::ExprKind::ReturnExpr(r) => self.collect_return(r),
            ast::ExprKind::BreakExpr(r) => self.collect_break(r),
            ast::ExprKind::BlockExpr(b) => self.collect_block(b),
            ast::ExprKind::Literal(e) => match e.kind() {
                ast::LiteralKind::Bool => {
                    let lit = Literal::Bool(e.token().kind() == T![true]);
                    self.alloc_expr(Expr::Literal(lit), syntax_ptr)
                }
                ast::LiteralKind::IntNumber => {
                    let (text, suffix) = e.text_and_suffix();
                    let (lit, errors) = integer_lit(&text, suffix.as_ref().map(SmolStr::as_str));
                    let expr_id = self.alloc_expr(Expr::Literal(lit), syntax_ptr);

                    for err in errors {
                        self.diagnostics
                            .push(ExprDiagnostic::LiteralError { expr: expr_id, err })
                    }

                    expr_id
                }
                ast::LiteralKind::FloatNumber => {
                    let (text, suffix) = e.text_and_suffix();
                    let (lit, errors) = float_lit(&text, suffix.as_ref().map(SmolStr::as_str));
                    let expr_id = self.alloc_expr(Expr::Literal(lit), syntax_ptr);

                    for err in errors {
                        self.diagnostics
                            .push(ExprDiagnostic::LiteralError { expr: expr_id, err })
                    }

                    expr_id
                }
                ast::LiteralKind::String => {
                    let lit = Literal::String(Default::default());
                    self.alloc_expr(Expr::Literal(lit), syntax_ptr)
                }
            },
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
                        | op @ BinOp::Multiply
                        | op @ BinOp::Divide
                        | op @ BinOp::Remainder
                        | op @ BinOp::LeftShift
                        | op @ BinOp::RightShift
                        | op @ BinOp::BitwiseAnd
                        | op @ BinOp::BitwiseOr
                        | op @ BinOp::BitwiseXor
                        | op @ BinOp::BooleanAnd
                        | op @ BinOp::BooleanOr
                        | op @ BinOp::Equals
                        | op @ BinOp::NotEqual
                        | op @ BinOp::Less
                        | op @ BinOp::LessEqual
                        | op @ BinOp::Greater
                        | op @ BinOp::GreatEqual => {
                            let op = match op {
                                BinOp::Add => BinaryOp::ArithOp(ArithOp::Add),
                                BinOp::Subtract => BinaryOp::ArithOp(ArithOp::Subtract),
                                BinOp::Multiply => BinaryOp::ArithOp(ArithOp::Multiply),
                                BinOp::Divide => BinaryOp::ArithOp(ArithOp::Divide),
                                BinOp::Remainder => BinaryOp::ArithOp(ArithOp::Remainder),
                                BinOp::LeftShift => BinaryOp::ArithOp(ArithOp::LeftShift),
                                BinOp::RightShift => BinaryOp::ArithOp(ArithOp::RightShift),
                                BinOp::BitwiseAnd => BinaryOp::ArithOp(ArithOp::BitAnd),
                                BinOp::BitwiseOr => BinaryOp::ArithOp(ArithOp::BitOr),
                                BinOp::BitwiseXor => BinaryOp::ArithOp(ArithOp::BitXor),
                                //BinOp::Power => BinaryOp::ArithOp(ArithOp::Power),
                                BinOp::BooleanAnd => BinaryOp::LogicOp(LogicOp::And),
                                BinOp::BooleanOr => BinaryOp::LogicOp(LogicOp::Or),
                                BinOp::Equals => BinaryOp::CmpOp(CmpOp::Eq { negated: false }),
                                BinOp::NotEqual => BinaryOp::CmpOp(CmpOp::Eq { negated: true }),
                                BinOp::Less => BinaryOp::CmpOp(CmpOp::Ord {
                                    ordering: Ordering::Less,
                                    strict: true,
                                }),
                                BinOp::LessEqual => BinaryOp::CmpOp(CmpOp::Ord {
                                    ordering: Ordering::Less,
                                    strict: false,
                                }),
                                BinOp::Greater => BinaryOp::CmpOp(CmpOp::Ord {
                                    ordering: Ordering::Greater,
                                    strict: true,
                                }),
                                BinOp::GreatEqual => BinaryOp::CmpOp(CmpOp::Ord {
                                    ordering: Ordering::Greater,
                                    strict: false,
                                }),
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
                        op @ BinOp::Assign
                        | op @ BinOp::AddAssign
                        | op @ BinOp::SubtractAssign
                        | op @ BinOp::MultiplyAssign
                        | op @ BinOp::DivideAssign
                        | op @ BinOp::RemainderAssign
                        | op @ BinOp::LeftShiftAssign
                        | op @ BinOp::RightShiftAssign
                        | op @ BinOp::BitAndAssign
                        | op @ BinOp::BitOrAssign
                        | op @ BinOp::BitXorAssign => {
                            let assign_op = match op {
                                BinOp::Assign => None,
                                BinOp::AddAssign => Some(ArithOp::Add),
                                BinOp::SubtractAssign => Some(ArithOp::Subtract),
                                BinOp::MultiplyAssign => Some(ArithOp::Multiply),
                                BinOp::DivideAssign => Some(ArithOp::Divide),
                                BinOp::RemainderAssign => Some(ArithOp::Remainder),
                                BinOp::LeftShiftAssign => Some(ArithOp::LeftShift),
                                BinOp::RightShiftAssign => Some(ArithOp::RightShift),
                                BinOp::BitAndAssign => Some(ArithOp::BitAnd),
                                BinOp::BitOrAssign => Some(ArithOp::BitOr),
                                BinOp::BitXorAssign => Some(ArithOp::BitXor),
                                _ => unreachable!("invalid assignment operator"),
                            };

                            let lhs = self.collect_expr_opt(e.lhs());
                            let rhs = self.collect_expr_opt(e.rhs());
                            self.alloc_expr(
                                Expr::BinaryOp {
                                    lhs,
                                    rhs,
                                    op: Some(BinaryOp::Assignment { op: assign_op }),
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
            ast::ExprKind::RecordLit(e) => {
                let type_id = self
                    .type_ref_builder
                    .alloc_from_node_opt(e.type_ref().as_ref());
                let mut field_ptrs = Vec::new();
                let record_lit = if let Some(r) = e.record_field_list() {
                    let fields = r
                        .fields()
                        .inspect(|field| field_ptrs.push(AstPtr::new(field)))
                        .map(|field| RecordLitField {
                            name: field
                                .name_ref()
                                .map(|nr| nr.as_name())
                                .unwrap_or_else(Name::missing),
                            expr: if let Some(e) = field.expr() {
                                self.collect_expr(e)
                            } else if let Some(nr) = field.name_ref() {
                                self.alloc_expr_field_shorthand(
                                    Expr::Path(Path::from_name_ref(&nr)),
                                    AstPtr::new(&field),
                                )
                            } else {
                                self.missing_expr()
                            },
                        })
                        .collect();
                    let spread = r.spread().map(|s| self.collect_expr(s));
                    Expr::RecordLit {
                        type_id,
                        fields,
                        spread,
                    }
                } else {
                    Expr::RecordLit {
                        type_id,
                        fields: Vec::new(),
                        spread: None,
                    }
                };

                let res = self.alloc_expr(record_lit, syntax_ptr);
                for (idx, ptr) in field_ptrs.into_iter().enumerate() {
                    self.source_map.field_map.insert((res, idx), ptr);
                }
                res
            }
            ast::ExprKind::FieldExpr(e) => {
                let expr = self.collect_expr_opt(e.expr());
                let name = match e.field_access() {
                    Some(kind) => kind.as_name(),
                    None => Name::missing(),
                };
                self.alloc_expr(Expr::Field { expr, name }, syntax_ptr)
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

                let condition = self.collect_condition_opt(e.condition());

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
                let src = Either::Left(syntax_ptr);
                self.source_map.expr_map.insert(src, inner);
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

    fn collect_condition_opt(&mut self, cond: Option<ast::Condition>) -> ExprId {
        if let Some(cond) = cond {
            self.collect_condition(cond)
        } else {
            self.exprs.alloc(Expr::Missing)
        }
    }

    fn collect_condition(&mut self, cond: ast::Condition) -> ExprId {
        match cond.pat() {
            None => self.collect_expr_opt(cond.expr()),
            _ => unreachable!("patterns in conditions are not yet supported"),
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

    fn collect_return(&mut self, expr: ast::ReturnExpr) -> ExprId {
        let syntax_node_ptr = AstPtr::new(&expr.clone().into());
        let expr = expr.expr().map(|e| self.collect_expr(e));
        self.alloc_expr(Expr::Return { expr }, syntax_node_ptr)
    }

    fn collect_break(&mut self, expr: ast::BreakExpr) -> ExprId {
        let syntax_node_ptr = AstPtr::new(&expr.clone().into());
        let expr = expr.expr().map(|e| self.collect_expr(e));
        self.alloc_expr(Expr::Break { expr }, syntax_node_ptr)
    }

    fn collect_loop(&mut self, expr: ast::LoopExpr) -> ExprId {
        let syntax_node_ptr = AstPtr::new(&expr.clone().into());
        let body = self.collect_block_opt(expr.loop_body());
        self.alloc_expr(Expr::Loop { body }, syntax_node_ptr)
    }

    fn collect_while(&mut self, expr: ast::WhileExpr) -> ExprId {
        let syntax_node_ptr = AstPtr::new(&expr.clone().into());
        let condition = self.collect_condition_opt(expr.condition());
        let body = self.collect_block_opt(expr.loop_body());
        self.alloc_expr(Expr::While { condition, body }, syntax_node_ptr)
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
            diagnostics: self.diagnostics,
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
            collector.collect_fn_body(&src.value)
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

/// Removes any underscores from a string if present
fn strip_underscores(s: &str) -> Cow<str> {
    if s.contains('_') {
        let mut s = s.to_string();
        s.retain(|c| c != '_');
        Cow::Owned(s)
    } else {
        Cow::Borrowed(s)
    }
}

/// Parses the given string into a float literal
fn float_lit(str: &str, suffix: Option<&str>) -> (Literal, Vec<LiteralError>) {
    let str = strip_underscores(str);
    filtered_float_lit(&str, suffix, 10)
}

/// Parses the given string into a float literal (underscores are already removed from str)
fn filtered_float_lit(str: &str, suffix: Option<&str>, base: u32) -> (Literal, Vec<LiteralError>) {
    let mut errors = Vec::new();
    if base != 10 {
        errors.push(LiteralError::NonDecimalFloat(base));
    }
    let kind = match suffix {
        Some(suf) => match BuiltinFloat::from_suffix(suf) {
            Some(suf) => LiteralFloatKind::Suffixed(suf),
            None => {
                errors.push(LiteralError::InvalidFloatSuffix(SmolStr::new(suf)));
                LiteralFloatKind::Unsuffixed
            }
        },
        None => LiteralFloatKind::Unsuffixed,
    };

    let value = if base == 10 {
        f64::from_str(str).expect("could not parse floating point number, this is definitely a bug")
    } else {
        0.0
    };
    (Literal::Float(LiteralFloat { kind, value }), errors)
}

/// Parses the given string into an integer literal
fn integer_lit(str: &str, suffix: Option<&str>) -> (Literal, Vec<LiteralError>) {
    let str = strip_underscores(str);

    let base = match str.as_bytes() {
        [b'0', b'x', ..] => 16,
        [b'0', b'o', ..] => 8,
        [b'0', b'b', ..] => 2,
        _ => 10,
    };

    let mut errors = Vec::new();

    let kind = match suffix {
        Some(suf) => match BuiltinInt::from_suffix(suf) {
            Some(ty) => LiteralIntKind::Suffixed(ty),
            None => {
                // 1f32 is a valid number, but its an integer disguised as a float
                if BuiltinFloat::from_suffix(suf).is_some() {
                    return filtered_float_lit(&str, suffix, base);
                }

                errors.push(LiteralError::InvalidIntSuffix(SmolStr::new(suf)));
                LiteralIntKind::Unsuffixed
            }
        },
        _ => LiteralIntKind::Unsuffixed,
    };

    let str = &str[if base != 10 { 2 } else { 0 }..];
    let (value, err) = match u128::from_str_radix(str, base) {
        Ok(i) => (i, None),
        Err(_) => {
            // Small bases are lexed as if they were base 10, e.g. the string might be
            // `0b10201`. This will cause the conversion above to fail.
            let from_lexer = base < 10
                && str
                    .chars()
                    .any(|c| c.to_digit(10).map_or(false, |d| d >= base));
            if from_lexer {
                (0, Some(LiteralError::LexerError))
            } else {
                (0, Some(LiteralError::IntTooLarge))
            }
        }
    };

    // TODO: Add check here to see if literal will fit given the suffix!

    if let Some(err) = err {
        errors.push(err);
    }

    (Literal::Int(LiteralInt { kind, value }), errors)
}

#[cfg(test)]
mod test {
    use crate::builtin_type::{BuiltinFloat, BuiltinInt};
    use crate::expr::{float_lit, LiteralError, LiteralFloat, LiteralFloatKind};
    use crate::expr::{integer_lit, LiteralInt, LiteralIntKind};
    use crate::Literal;
    use mun_syntax::SmolStr;

    #[test]
    fn test_integer_literals() {
        assert_eq!(
            integer_lit("12", None),
            (
                Literal::Int(LiteralInt {
                    kind: LiteralIntKind::Unsuffixed,
                    value: 12
                }),
                vec![]
            )
        );
        assert_eq!(
            integer_lit("0xF00BA", None),
            (
                Literal::Int(LiteralInt {
                    kind: LiteralIntKind::Unsuffixed,
                    value: 0xF00BA
                }),
                vec![]
            )
        );
        assert_eq!(
            integer_lit("10_000_000", None),
            (
                Literal::Int(LiteralInt {
                    kind: LiteralIntKind::Unsuffixed,
                    value: 10_000_000
                }),
                vec![]
            )
        );
        assert_eq!(
            integer_lit("0o765431", None),
            (
                Literal::Int(LiteralInt {
                    kind: LiteralIntKind::Unsuffixed,
                    value: 0o765431
                }),
                vec![]
            )
        );
        assert_eq!(
            integer_lit("0b01011100", None),
            (
                Literal::Int(LiteralInt {
                    kind: LiteralIntKind::Unsuffixed,
                    value: 0b01011100
                }),
                vec![]
            )
        );

        assert_eq!(
            integer_lit("0b02011100", None),
            (
                Literal::Int(LiteralInt {
                    kind: LiteralIntKind::Unsuffixed,
                    value: 0
                }),
                vec![LiteralError::LexerError]
            )
        );
        assert_eq!(
            integer_lit("0o09", None),
            (
                Literal::Int(LiteralInt {
                    kind: LiteralIntKind::Unsuffixed,
                    value: 0
                }),
                vec![LiteralError::LexerError]
            )
        );

        assert_eq!(
            integer_lit("1234", Some("foo")),
            (
                Literal::Int(LiteralInt {
                    kind: LiteralIntKind::Unsuffixed,
                    value: 1234
                }),
                vec![LiteralError::InvalidIntSuffix(SmolStr::new("foo"))]
            )
        );

        assert_eq!(
            integer_lit("123", Some("i8")),
            (
                Literal::Int(LiteralInt {
                    kind: LiteralIntKind::Suffixed(BuiltinInt::I8),
                    value: 123
                }),
                vec![]
            )
        );

        assert_eq!(
            integer_lit("1234", Some("i16")),
            (
                Literal::Int(LiteralInt {
                    kind: LiteralIntKind::Suffixed(BuiltinInt::I16),
                    value: 1234
                }),
                vec![]
            )
        );

        assert_eq!(
            integer_lit("1234", Some("i32")),
            (
                Literal::Int(LiteralInt {
                    kind: LiteralIntKind::Suffixed(BuiltinInt::I32),
                    value: 1234
                }),
                vec![]
            )
        );

        assert_eq!(
            integer_lit("1234", Some("i64")),
            (
                Literal::Int(LiteralInt {
                    kind: LiteralIntKind::Suffixed(BuiltinInt::I64),
                    value: 1234
                }),
                vec![]
            )
        );

        assert_eq!(
            integer_lit("1234", Some("i128")),
            (
                Literal::Int(LiteralInt {
                    kind: LiteralIntKind::Suffixed(BuiltinInt::I128),
                    value: 1234
                }),
                vec![]
            )
        );

        assert_eq!(
            integer_lit("1234", Some("isize")),
            (
                Literal::Int(LiteralInt {
                    kind: LiteralIntKind::Suffixed(BuiltinInt::ISIZE),
                    value: 1234
                }),
                vec![]
            )
        );

        assert_eq!(
            integer_lit("123", Some("u8")),
            (
                Literal::Int(LiteralInt {
                    kind: LiteralIntKind::Suffixed(BuiltinInt::U8),
                    value: 123
                }),
                vec![]
            )
        );

        assert_eq!(
            integer_lit("1234", Some("u16")),
            (
                Literal::Int(LiteralInt {
                    kind: LiteralIntKind::Suffixed(BuiltinInt::U16),
                    value: 1234
                }),
                vec![]
            )
        );

        assert_eq!(
            integer_lit("1234", Some("u32")),
            (
                Literal::Int(LiteralInt {
                    kind: LiteralIntKind::Suffixed(BuiltinInt::U32),
                    value: 1234
                }),
                vec![]
            )
        );

        assert_eq!(
            integer_lit("1234", Some("u64")),
            (
                Literal::Int(LiteralInt {
                    kind: LiteralIntKind::Suffixed(BuiltinInt::U64),
                    value: 1234
                }),
                vec![]
            )
        );

        assert_eq!(
            integer_lit("1234", Some("u128")),
            (
                Literal::Int(LiteralInt {
                    kind: LiteralIntKind::Suffixed(BuiltinInt::U128),
                    value: 1234
                }),
                vec![]
            )
        );

        assert_eq!(
            integer_lit("1234", Some("usize")),
            (
                Literal::Int(LiteralInt {
                    kind: LiteralIntKind::Suffixed(BuiltinInt::USIZE),
                    value: 1234
                }),
                vec![]
            )
        );

        assert_eq!(
            integer_lit("1", Some("f32")),
            (
                Literal::Float(LiteralFloat {
                    kind: LiteralFloatKind::Suffixed(BuiltinFloat::F32),
                    value: 1.0
                }),
                vec![]
            )
        );

        assert_eq!(
            integer_lit("0x1", Some("f32")),
            (
                Literal::Float(LiteralFloat {
                    kind: LiteralFloatKind::Suffixed(BuiltinFloat::F32),
                    value: 0.0
                }),
                vec![LiteralError::NonDecimalFloat(16)]
            )
        );
    }

    #[test]
    fn test_float_literals() {
        assert_eq!(
            float_lit("1234.1234", None),
            (
                Literal::Float(LiteralFloat {
                    kind: LiteralFloatKind::Unsuffixed,
                    value: 1234.1234
                }),
                vec![]
            )
        );

        assert_eq!(
            float_lit("1_234.1_234", None),
            (
                Literal::Float(LiteralFloat {
                    kind: LiteralFloatKind::Unsuffixed,
                    value: 1234.1234
                }),
                vec![]
            )
        );

        assert_eq!(
            float_lit("1234.1234e2", None),
            (
                Literal::Float(LiteralFloat {
                    kind: LiteralFloatKind::Unsuffixed,
                    value: 123412.34
                }),
                vec![]
            )
        );

        assert_eq!(
            float_lit("1234.1234e2", Some("foo")),
            (
                Literal::Float(LiteralFloat {
                    kind: LiteralFloatKind::Unsuffixed,
                    value: 123412.34
                }),
                vec![LiteralError::InvalidFloatSuffix(SmolStr::new("foo"))]
            )
        );

        assert_eq!(
            float_lit("1234.1234e2", Some("f32")),
            (
                Literal::Float(LiteralFloat {
                    kind: LiteralFloatKind::Suffixed(BuiltinFloat::F32),
                    value: 123412.34
                }),
                vec![]
            )
        );

        assert_eq!(
            float_lit("1234.1234e2", Some("f64")),
            (
                Literal::Float(LiteralFloat {
                    kind: LiteralFloatKind::Suffixed(BuiltinFloat::F64),
                    value: 123412.34
                }),
                vec![]
            )
        );
    }
}

mod diagnostics {
    use super::{ExprDiagnostic, LiteralError};
    use crate::code_model::DefWithBody;
    use crate::diagnostics::{
        DiagnosticSink, IntLiteralTooLarge, InvalidFloatingPointLiteral, InvalidLiteral,
        InvalidLiteralSuffix,
    };
    use crate::HirDatabase;

    impl ExprDiagnostic {
        pub(crate) fn add_to(
            &self,
            db: &impl HirDatabase,
            owner: DefWithBody,
            sink: &mut DiagnosticSink,
        ) {
            let source_map = owner.body_source_map(db);

            match self {
                ExprDiagnostic::LiteralError { expr, err } => {
                    let literal = source_map
                        .expr_syntax(*expr)
                        .expect("could not retrieve expr from source map")
                        .map(|expr_src| {
                            expr_src
                                .left()
                                .expect("could not retrieve expr from ExprSource")
                                .cast()
                                .expect("could not cast expression to literal")
                        });
                    match err {
                        LiteralError::IntTooLarge => sink.push(IntLiteralTooLarge { literal }),
                        LiteralError::LexerError => sink.push(InvalidLiteral { literal }),
                        LiteralError::InvalidIntSuffix(suffix) => sink.push(InvalidLiteralSuffix {
                            literal,
                            suffix: suffix.clone(),
                        }),
                        LiteralError::InvalidFloatSuffix(suffix) => {
                            sink.push(InvalidLiteralSuffix {
                                literal,
                                suffix: suffix.clone(),
                            })
                        }
                        LiteralError::NonDecimalFloat(base) => {
                            sink.push(InvalidFloatingPointLiteral {
                                literal,
                                base: *base,
                            })
                        }
                    }
                }
            }
        }
    }
}

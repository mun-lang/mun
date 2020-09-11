use crate::code_model::StructKind;
use crate::in_file::InFile;
use crate::{FileId, HirDatabase, IntTy, Name, Ty};
use mun_syntax::{ast, AstPtr, SmolStr, SyntaxNode, SyntaxNodePtr, TextRange};
use std::{any::Any, fmt};

/// Diagnostic defines hir API for errors and warnings.
///
/// It is used as a `dyn` object, which you can downcast to concrete diagnostics. DiagnosticSink
/// are structured, meaning that they include rich information which can be used by IDE to create
/// fixes.
///
/// Internally, various subsystems of HIR produce diagnostics specific to a subsystem (typically,
/// an `enum`), which are safe to store in salsa but do not include source locations. Such internal
/// diagnostics are transformed into an instance of `Diagnostic` on demand.
pub trait Diagnostic: Any + Send + Sync + fmt::Debug + 'static {
    fn message(&self) -> String;
    fn source(&self) -> InFile<SyntaxNodePtr>;
    fn highlight_range(&self) -> TextRange {
        self.source().value.range()
    }
    fn as_any(&self) -> &(dyn Any + Send + 'static);
}

pub trait AstDiagnostic {
    type AST;
    fn ast(&self, db: &dyn HirDatabase) -> Self::AST;
}

impl dyn Diagnostic {
    pub fn syntax_node(&self, db: &dyn HirDatabase) -> SyntaxNode {
        let node = db.parse(self.source().file_id).syntax_node();
        self.source().value.to_node(&node)
    }

    pub fn downcast_ref<D: Diagnostic>(&self) -> Option<&D> {
        self.as_any().downcast_ref()
    }
}

type DiagnosticCallback<'a> = Box<dyn FnMut(&dyn Diagnostic) -> Result<(), ()> + 'a>;

pub struct DiagnosticSink<'a> {
    callbacks: Vec<DiagnosticCallback<'a>>,
    default_callback: Box<dyn FnMut(&dyn Diagnostic) + 'a>,
}

impl<'a> DiagnosticSink<'a> {
    pub fn new(cb: impl FnMut(&dyn Diagnostic) + 'a) -> DiagnosticSink<'a> {
        DiagnosticSink {
            callbacks: Vec::new(),
            default_callback: Box::new(cb),
        }
    }

    pub fn on<D: Diagnostic, F: FnMut(&D) + 'a>(mut self, mut cb: F) -> DiagnosticSink<'a> {
        let cb = move |diag: &dyn Diagnostic| match diag.downcast_ref::<D>() {
            Some(d) => {
                cb(d);
                Ok(())
            }
            None => Err(()),
        };
        self.callbacks.push(Box::new(cb));
        self
    }

    pub(crate) fn push(&mut self, d: impl Diagnostic) {
        let d: &dyn Diagnostic = &d;
        for cb in self.callbacks.iter_mut() {
            match cb(d) {
                Ok(()) => return,
                Err(()) => (),
            }
        }
        (self.default_callback)(d)
    }
}

#[derive(Debug)]
pub struct UnresolvedValue {
    pub file: FileId,
    pub expr: SyntaxNodePtr,
}

impl Diagnostic for UnresolvedValue {
    fn message(&self) -> String {
        "undefined value".to_string()
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        InFile::new(self.file, self.expr)
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

#[derive(Debug)]
pub struct UnresolvedType {
    pub file: FileId,
    pub type_ref: AstPtr<ast::TypeRef>,
}

impl Diagnostic for UnresolvedType {
    fn message(&self) -> String {
        "undefined type".to_string()
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        InFile::new(self.file, self.type_ref.syntax_node_ptr())
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

#[derive(Debug)]
pub struct CyclicType {
    pub file: FileId,
    pub type_ref: AstPtr<ast::TypeRef>,
}

impl Diagnostic for CyclicType {
    fn message(&self) -> String {
        "cyclic type".to_string()
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        InFile::new(self.file, self.type_ref.syntax_node_ptr())
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

#[derive(Debug)]
pub struct PrivateAccess {
    pub file: FileId,
    pub expr: SyntaxNodePtr,
}

impl Diagnostic for PrivateAccess {
    fn message(&self) -> String {
        "access of private type".to_string()
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        InFile::new(self.file, self.expr)
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

#[derive(Debug)]
pub struct ExpectedFunction {
    pub file: FileId,
    pub expr: SyntaxNodePtr,
    pub found: Ty,
}

impl Diagnostic for ExpectedFunction {
    fn message(&self) -> String {
        "expected function type".to_string()
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        InFile::new(self.file, self.expr)
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

#[derive(Debug)]
pub struct ParameterCountMismatch {
    pub file: FileId,
    pub expr: SyntaxNodePtr,
    pub expected: usize,
    pub found: usize,
}

impl Diagnostic for ParameterCountMismatch {
    fn message(&self) -> String {
        format!(
            "this function takes {} parameters but {} parameters was supplied",
            self.expected, self.found
        )
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        InFile::new(self.file, self.expr)
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

#[derive(Debug)]
pub struct MismatchedType {
    pub file: FileId,
    pub expr: SyntaxNodePtr,
    pub expected: Ty,
    pub found: Ty,
}

impl Diagnostic for MismatchedType {
    fn message(&self) -> String {
        "mismatched type".to_string()
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        InFile::new(self.file, self.expr)
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

#[derive(Debug)]
pub struct IncompatibleBranch {
    pub file: FileId,
    pub if_expr: SyntaxNodePtr,
    pub expected: Ty,
    pub found: Ty,
}

impl Diagnostic for IncompatibleBranch {
    fn message(&self) -> String {
        "mismatched branches".to_string()
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        InFile::new(self.file, self.if_expr)
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

#[derive(Debug)]
pub struct InvalidLHS {
    /// The file that contains the expressions
    pub file: FileId,

    /// The expression containing the `lhs`
    pub expr: SyntaxNodePtr,

    /// The left-hand side of the expression.
    pub lhs: SyntaxNodePtr,
}

impl Diagnostic for InvalidLHS {
    fn message(&self) -> String {
        "invalid left hand side of expression".to_string()
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        InFile::new(self.file, self.lhs)
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

#[derive(Debug)]
pub struct MissingElseBranch {
    pub file: FileId,
    pub if_expr: SyntaxNodePtr,
    pub found: Ty,
}

impl Diagnostic for MissingElseBranch {
    fn message(&self) -> String {
        "missing else branch".to_string()
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        InFile::new(self.file, self.if_expr)
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

#[derive(Debug)]
pub struct CannotApplyBinaryOp {
    pub file: FileId,
    pub expr: SyntaxNodePtr,
    pub lhs: Ty,
    pub rhs: Ty,
}

impl Diagnostic for CannotApplyBinaryOp {
    fn message(&self) -> String {
        "cannot apply binary operator".to_string()
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        InFile::new(self.file, self.expr)
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

#[derive(Debug)]
pub struct CannotApplyUnaryOp {
    pub file: FileId,
    pub expr: SyntaxNodePtr,
    pub ty: Ty,
}

impl Diagnostic for CannotApplyUnaryOp {
    fn message(&self) -> String {
        "cannot apply unary operator".to_string()
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        InFile::new(self.file, self.expr)
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

#[derive(Debug)]
pub struct DuplicateDefinition {
    pub file: FileId,
    pub name: String,
    pub first_definition: SyntaxNodePtr,
    pub definition: SyntaxNodePtr,
}

impl Diagnostic for DuplicateDefinition {
    fn message(&self) -> String {
        format!("the name `{}` is defined multiple times", self.name)
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        InFile::new(self.file, self.definition)
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

#[derive(Debug)]
pub struct ReturnMissingExpression {
    pub file: FileId,
    pub return_expr: SyntaxNodePtr,
}

impl Diagnostic for ReturnMissingExpression {
    fn message(&self) -> String {
        "`return;` in a function whose return type is not `()`".to_owned()
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        InFile::new(self.file, self.return_expr)
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

#[derive(Debug)]
pub struct BreakOutsideLoop {
    pub file: FileId,
    pub break_expr: SyntaxNodePtr,
}

impl Diagnostic for BreakOutsideLoop {
    fn message(&self) -> String {
        "`break` outside of a loop".to_owned()
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        InFile::new(self.file, self.break_expr)
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

#[derive(Debug)]
pub struct BreakWithValueOutsideLoop {
    pub file: FileId,
    pub break_expr: SyntaxNodePtr,
}

impl Diagnostic for BreakWithValueOutsideLoop {
    fn message(&self) -> String {
        "`break` with value can only appear in a `loop`".to_owned()
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        InFile::new(self.file, self.break_expr)
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

#[derive(Debug)]
pub struct AccessUnknownField {
    pub file: FileId,
    pub expr: SyntaxNodePtr,
    pub receiver_ty: Ty,
    pub name: Name,
}

impl Diagnostic for AccessUnknownField {
    fn message(&self) -> String {
        "attempted to access a non-existent field in a struct.".to_string()
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        InFile::new(self.file, self.expr)
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

#[derive(Debug)]
pub struct FieldCountMismatch {
    pub file: FileId,
    pub expr: SyntaxNodePtr,
    pub expected: usize,
    pub found: usize,
}

impl Diagnostic for FieldCountMismatch {
    fn message(&self) -> String {
        format!(
            "this tuple struct literal has {} field{} but {} field{} supplied",
            self.expected,
            if self.expected == 1 { "" } else { "s" },
            self.found,
            if self.found == 1 { " was" } else { "s were" },
        )
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        InFile::new(self.file, self.expr)
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

#[derive(Debug)]
pub struct MissingFields {
    pub file: FileId,
    pub fields: SyntaxNodePtr,
    pub struct_ty: Ty,
    pub field_names: Vec<Name>,
}

impl Diagnostic for MissingFields {
    fn message(&self) -> String {
        use std::fmt::Write;
        let mut message = "missing record fields:\n".to_string();
        for field in &self.field_names {
            writeln!(message, "- {}", field).unwrap();
        }
        message
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        InFile::new(self.file, self.fields)
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

#[derive(Debug)]
pub struct MismatchedStructLit {
    pub file: FileId,
    pub expr: SyntaxNodePtr,
    pub expected: StructKind,
    pub found: StructKind,
}

impl Diagnostic for MismatchedStructLit {
    fn message(&self) -> String {
        format!(
            "mismatched struct literal kind. expected `{}`, found `{}`",
            self.expected, self.found
        )
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        InFile::new(self.file, self.expr)
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

#[derive(Debug)]
pub struct NoFields {
    pub file: FileId,
    pub receiver_expr: SyntaxNodePtr,
    pub found: Ty,
}

impl Diagnostic for NoFields {
    fn message(&self) -> String {
        "attempted to access a field on a primitive type.".to_string()
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        InFile::new(self.file, self.receiver_expr)
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}
#[derive(Debug)]
pub struct NoSuchField {
    pub file: FileId,
    pub field: SyntaxNodePtr,
}

impl Diagnostic for NoSuchField {
    fn message(&self) -> String {
        "no such field".to_string()
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        InFile::new(self.file, self.field)
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

#[derive(Debug)]
pub struct PossiblyUninitializedVariable {
    pub file: FileId,
    pub pat: SyntaxNodePtr,
}

impl Diagnostic for PossiblyUninitializedVariable {
    fn message(&self) -> String {
        "use of possibly-uninitialized variable".to_string()
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        InFile::new(self.file, self.pat)
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

#[derive(Debug)]
pub struct ExternCannotHaveBody {
    pub func: InFile<SyntaxNodePtr>,
}

impl Diagnostic for ExternCannotHaveBody {
    fn message(&self) -> String {
        "extern functions cannot have bodies".to_string()
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        self.func
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

#[derive(Debug)]
pub struct ExternNonPrimitiveParam {
    pub param: InFile<SyntaxNodePtr>,
}

impl Diagnostic for ExternNonPrimitiveParam {
    fn message(&self) -> String {
        "extern functions can only have primitives as parameter- and return types".to_string()
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        self.param
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

/// An error that is emitted if a literal is too large to even parse
#[derive(Debug)]
pub struct IntLiteralTooLarge {
    pub literal: InFile<AstPtr<ast::Literal>>,
}

impl Diagnostic for IntLiteralTooLarge {
    fn message(&self) -> String {
        "int literal is too large".to_owned()
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        self.literal.map(|ptr| ptr.into())
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

/// An error that is emitted if a literal is too large for its suffix
#[derive(Debug)]
pub struct LiteralOutOfRange {
    pub literal: InFile<AstPtr<ast::Literal>>,
    pub int_ty: IntTy,
}

impl Diagnostic for LiteralOutOfRange {
    fn message(&self) -> String {
        format!("literal out of range for `{}`", self.int_ty.as_str())
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        self.literal.map(|ptr| ptr.into())
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

/// An error that is emitted for a literal with an invalid suffix (e.g. `123_foo`)
#[derive(Debug)]
pub struct InvalidLiteralSuffix {
    pub literal: InFile<AstPtr<ast::Literal>>,
    pub suffix: SmolStr,
}

impl Diagnostic for InvalidLiteralSuffix {
    fn message(&self) -> String {
        format!("invalid suffix `{}`", self.suffix)
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        self.literal.map(|ptr| ptr.into())
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

/// An error that is emitted for a literal with a floating point suffix with a non 10 base (e.g.
/// `0x123_f32`)
#[derive(Debug)]
pub struct InvalidFloatingPointLiteral {
    pub literal: InFile<AstPtr<ast::Literal>>,
    pub base: u32,
}

impl Diagnostic for InvalidFloatingPointLiteral {
    fn message(&self) -> String {
        match self.base {
            2 => "binary float literal is not supported".to_owned(),
            8 => "octal float literal is not supported".to_owned(),
            16 => "hexadecimal float literal is not supported".to_owned(),
            _ => "unsupported base for floating pointer literal".to_owned(),
        }
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        self.literal.map(|ptr| ptr.into())
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

/// An error that is emitted for a malformed literal (e.g. `0b22222`)
#[derive(Debug)]
pub struct InvalidLiteral {
    pub literal: InFile<AstPtr<ast::Literal>>,
}

impl Diagnostic for InvalidLiteral {
    fn message(&self) -> String {
        "invalid literal value".to_owned()
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        self.literal.map(|ptr| ptr.into())
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

#[derive(Debug)]
pub struct FreeTypeAliasWithoutTypeRef {
    pub type_alias_def: InFile<SyntaxNodePtr>,
}

impl Diagnostic for FreeTypeAliasWithoutTypeRef {
    fn message(&self) -> String {
        "free type alias without type ref".to_string()
    }

    fn source(&self) -> InFile<SyntaxNodePtr> {
        self.type_alias_def
    }

    fn as_any(&self) -> &(dyn Any + Send + 'static) {
        self
    }
}

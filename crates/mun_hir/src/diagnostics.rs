use crate::in_file::InFile;
use crate::{FileId, HirDatabase, Ty};
use mun_syntax::{ast, AstPtr, SyntaxNode, SyntaxNodePtr, TextRange};
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
    fn ast(&self, db: &impl HirDatabase) -> Self::AST;
}

impl dyn Diagnostic {
    pub fn syntax_node(&self, db: &impl HirDatabase) -> SyntaxNode {
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

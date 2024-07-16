use mun_hir::InFile;
use mun_syntax::{ast, AstNode, Parse, SourceFile, SyntaxKind, SyntaxNodePtr, TextRange};

use crate::{Diagnostic, SecondaryAnnotation, SourceAnnotation};

/// For a given node returns the signature range (if that is applicable for the
/// type of node)
/// ```rust, ignore
/// fn foo_bar() {
/// ^^^^^^^^^^^^___ this part
///     // ...
/// }
/// ```
/// or
/// ```rust, ignore
/// pub(gc) struct Foo {
///         ^^^^^^^^^^___ this part
///     // ...
/// }
/// ```
///
/// If the specified syntax node is not a function definition or structure
/// definition, returns the range of the syntax node itself.
fn syntax_node_signature_range(
    syntax_node_ptr: &SyntaxNodePtr,
    parse: &Parse<SourceFile>,
) -> TextRange {
    match syntax_node_ptr.kind() {
        SyntaxKind::FUNCTION_DEF => {
            ast::FunctionDef::cast(syntax_node_ptr.to_node(parse.tree().syntax()))
                .map_or_else(|| syntax_node_ptr.range(), |f| f.signature_range())
        }
        SyntaxKind::STRUCT_DEF => {
            ast::StructDef::cast(syntax_node_ptr.to_node(parse.tree().syntax()))
                .map_or_else(|| syntax_node_ptr.range(), |s| s.signature_range())
        }
        SyntaxKind::TYPE_ALIAS_DEF => {
            ast::TypeAliasDef::cast(syntax_node_ptr.to_node(parse.tree().syntax()))
                .map_or_else(|| syntax_node_ptr.range(), |s| s.signature_range())
        }
        _ => syntax_node_ptr.range(),
    }
}

/// For a given node returns the identifier range (if that is applicable for the
/// type of node)
///  ```rust, ignore
/// fn foo_bar() {
///    ^^^^^^^___ this part
///     // ...
/// }
/// ```
/// or
/// ```rust, ignore
/// pub(gc) struct Foo {
///                ^^^___ this part
///     // ...
/// }
/// ```
///
/// If the specified syntax node is not a function definition or structure
/// definition, returns the range of the syntax node itself.
fn syntax_node_identifier_range(
    syntax_node_ptr: &SyntaxNodePtr,
    parse: &Parse<SourceFile>,
) -> TextRange {
    match syntax_node_ptr.kind() {
        SyntaxKind::FUNCTION_DEF | SyntaxKind::STRUCT_DEF | SyntaxKind::TYPE_ALIAS_DEF => {
            syntax_node_ptr
                .to_node(parse.tree().syntax())
                .children()
                .find(|n| n.kind() == SyntaxKind::NAME)
                .map_or_else(|| syntax_node_ptr.range(), |name| name.text_range())
        }
        _ => syntax_node_ptr.range(),
    }
}

/// An error that is emitted when a duplication definition is encountered:
///
/// ```mun
/// struct Foo {
///     b: i32
/// }
///
/// struct Foo {    // Duplicate definition
///     a: i32
/// }
/// ```
pub struct DuplicateDefinition<'db, 'diag, DB: mun_hir::HirDatabase> {
    db: &'db DB,
    diag: &'diag mun_hir::diagnostics::DuplicateDefinition,
}

impl<'db, 'diag, DB: mun_hir::HirDatabase> Diagnostic for DuplicateDefinition<'db, 'diag, DB> {
    fn range(&self) -> TextRange {
        syntax_node_identifier_range(
            &self.diag.definition.value,
            &self.db.parse(self.diag.definition.file_id),
        )
    }

    fn title(&self) -> String {
        format!(
            "a {} named `{}` has already been defined in this module",
            self.value_or_type_string(),
            self.diag.name,
        )
    }

    fn primary_annotation(&self) -> Option<SourceAnnotation> {
        Some(SourceAnnotation {
            range: syntax_node_signature_range(
                &self.diag.definition.value,
                &self.db.parse(self.diag.definition.file_id),
            ),
            message: format!("`{}` redefined here", self.diag.name),
        })
    }

    fn secondary_annotations(&self) -> Vec<SecondaryAnnotation> {
        vec![SecondaryAnnotation {
            range: InFile::new(
                self.diag.first_definition.file_id,
                syntax_node_signature_range(
                    &self.diag.first_definition.value,
                    &self.db.parse(self.diag.first_definition.file_id),
                ),
            ),
            message: format!(
                "first definition of the {} `{}` here",
                self.value_or_type_string(),
                self.diag.name
            ),
        }]
    }

    fn footer(&self) -> Vec<String> {
        vec![format!(
            "`{}` must be defined only once in the {} namespace of this module",
            self.diag.name,
            self.value_or_type_string()
        )]
    }
}

impl<'db, 'diag, DB: mun_hir::HirDatabase> DuplicateDefinition<'db, 'diag, DB> {
    /// Returns either `type` or `value` definition on the type of definition.
    fn value_or_type_string(&self) -> &'static str {
        if self.diag.definition.value.kind() == SyntaxKind::STRUCT_DEF {
            "type"
        } else {
            "value"
        }
    }
}

impl<'db, 'diag, DB: mun_hir::HirDatabase> DuplicateDefinition<'db, 'diag, DB> {
    /// Constructs a new instance of `DuplicateDefinition`
    pub fn new(db: &'db DB, diag: &'diag mun_hir::diagnostics::DuplicateDefinition) -> Self {
        DuplicateDefinition { db, diag }
    }
}

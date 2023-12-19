//! This module contains the validation pass for the AST. See the [`validate`] function for more
//! information.

use crate::ast::VisibilityOwner;
use crate::{ast, ast::AstNode, match_ast, SyntaxError, SyntaxNode};

/// A validation pass that checks that the AST is valid.
///
/// Even though the AST could be valid (aka without parse errors), it could still be semantically
/// incorrect. For example, a struct cannot be declared in an impl block. This pass checks for
/// these kinds of errors.
pub(crate) fn validate(root: &SyntaxNode) -> Vec<SyntaxError> {
    let mut errors = Vec::new();
    for node in root.descendants() {
        match_ast! {
            match node {
                ast::Impl(it) => validate_impl(it, &mut errors),
                _ => (),
            }
        }
    }

    errors
}

/// Validates the semantic validity of an `impl` block.
fn validate_impl(node: ast::Impl, errors: &mut Vec<SyntaxError>) {
    validate_impl_visibility(node.clone(), errors);
    validate_impl_associated_items(node, errors);
}

/// Validate that the visibility of an impl block is undefined.
fn validate_impl_visibility(node: ast::Impl, errors: &mut Vec<SyntaxError>) {
    if let Some(vis) = node.visibility() {
        errors.push(SyntaxError::parse_error(
            "visibility is not allowed on impl blocks",
            vis.syntax.text_range(),
        ));
    }
}

/// Validate that only valid items are declared in an impl block. For example, a struct
/// cannot be declared in an impl block.
fn validate_impl_associated_items(node: ast::Impl, errors: &mut Vec<SyntaxError>) {
    let Some(assoc_items) = node.associated_item_list() else {
        return;
    };

    for item in assoc_items.syntax.children() {
        match_ast! {
            match item {
                ast::FunctionDef(_it) => (),
                _ => errors.push(SyntaxError::parse_error("only functions are allowed in impl blocks", item.text_range())),
            }
        }
    }
}

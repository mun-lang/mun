//! Syntax Tree Library used throughout Mun.
//!
//! This crate is heavily inspired by Rust Analyzers ra_syntax crate.
//!
//! Properties:
//!     - easy and fast incremental re-parsing
//!     - graceful handling of errors
//!     - full-fidelity representation (*any* text can be precisely represented as
//!       a syntax tree)

#[macro_use]
mod syntax_kind;

pub mod ast;
mod parsing;
mod ptr;
mod syntax_error;
mod syntax_node;

#[cfg(test)]
mod tests;

use std::{fmt::Write, marker::PhantomData, sync::Arc};

pub use crate::{
    ast::AstNode,
    parsing::{tokenize, Token},
    ptr::{AstPtr, SyntaxNodePtr},
    syntax_error::{SyntaxError, SyntaxErrorKind},
    syntax_kind::SyntaxKind,
    syntax_node::{Direction, SyntaxElement, SyntaxNode, SyntaxToken, SyntaxTreeBuilder},
};
pub use rowan::{SmolStr, TextRange, TextUnit};

use rowan::GreenNode;

/// `Parse` is the result of the parsing: a syntax tree and a collection of errors.
///
/// Note that we always produce a syntax tree, event for completely invalid files.
#[derive(Debug, PartialEq, Eq)]
pub struct Parse<T> {
    green: GreenNode,
    errors: Arc<Vec<SyntaxError>>,
    _ty: PhantomData<fn() -> T>,
}

impl<T> Clone for Parse<T> {
    fn clone(&self) -> Parse<T> {
        Parse {
            green: self.green.clone(),
            errors: self.errors.clone(),
            _ty: PhantomData,
        }
    }
}

impl<T> Parse<T> {
    fn new(green: GreenNode, errors: Vec<SyntaxError>) -> Parse<T> {
        Parse {
            green,
            errors: Arc::new(errors),
            _ty: PhantomData,
        }
    }

    pub fn syntax_node(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green.clone())
    }
}

impl<T: AstNode> Parse<T> {
    pub fn to_syntax(self) -> Parse<SyntaxNode> {
        Parse {
            green: self.green,
            errors: self.errors,
            _ty: PhantomData,
        }
    }

    pub fn tree(&self) -> T {
        T::cast(self.syntax_node()).unwrap()
    }

    pub fn errors(&self) -> &[SyntaxError] {
        &*self.errors
    }

    pub fn ok(self) -> Result<T, Arc<Vec<SyntaxError>>> {
        if self.errors.is_empty() {
            Ok(self.tree())
        } else {
            Err(self.errors)
        }
    }
}

impl Parse<SyntaxNode> {
    pub fn cast<N: AstNode>(self) -> Option<Parse<N>> {
        if N::cast(self.syntax_node()).is_some() {
            Some(Parse {
                green: self.green,
                errors: self.errors,
                _ty: PhantomData,
            })
        } else {
            None
        }
    }
}

impl Parse<SourceFile> {
    pub fn debug_dump(&self) -> String {
        let mut buf = format!("{:#?}", self.tree().syntax());
        for err in self.errors.iter() {
            writeln!(buf, "error {:?}: {}", err.location(), err.kind()).unwrap();
        }
        buf
    }
}

/// `SourceFile` represents a parse tree for a single Mun file.
pub use crate::ast::SourceFile;

impl SourceFile {
    pub fn parse(text: &str) -> Parse<SourceFile> {
        let (green, errors) = parsing::parse_text(text);
        //errors.extend(validation::validate(&SourceFile::new(green.clone())));
        Parse {
            green,
            errors: Arc::new(errors),
            _ty: PhantomData,
        }
    }
}

/// This tests does not assert anything and instead just shows off the crate's API.
#[test]
fn api_walkthrough() {
    use ast::ModuleItemOwner;
    use ast::NameOwner;

    let source_code = "
        fn foo() {

        }
    ";

    // `SourceFile` is the main entry point.
    //
    // The `parse` method returns a `Parse` -- a pair of syntax tree and a list of errors. That is,
    // syntax tree is constructed even in presence of errors.
    let parse = SourceFile::parse(source_code);
    assert!(parse.errors().is_empty());

    // The `tree` method returns an owned syntax node of type `SourceFile`.
    // Owned nodes are cheap: inside, they are `Rc` handles to the underlying data.
    let file: SourceFile = parse.tree();

    // `SourceFile` is the root of the syntax tree. We can iterate file's items:
    let mut func = None;
    for item in file.items() {
        match item.kind() {
            ast::ModuleItemKind::FunctionDef(f) => func = Some(f),
        }
    }

    // The returned items are always references.
    let func: ast::FunctionDef = func.unwrap();

    // Each AST node has a bunch of getters for children. All getters return `Option`s though, to
    // account for incomplete code. Some getters are common for several kinds of node. In this case,
    // a trait like `ast::NameOwner` usually exists. By convention, all ast types should be used
    // with `ast::` qualifier.
    let name: Option<ast::Name> = func.name();
    let name = name.unwrap();
    assert_eq!(name.text(), "foo");
}

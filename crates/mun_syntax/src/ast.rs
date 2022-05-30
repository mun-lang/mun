mod expr_extensions;
#[macro_use]
mod extensions;
mod generated;
mod token_extensions;
mod tokens;
mod traits;

use crate::{syntax_node::SyntaxNodeChildren, SyntaxKind, SyntaxNode, SyntaxToken};

pub use self::{
    expr_extensions::*,
    extensions::{PathSegmentKind, StructKind, VisibilityKind},
    generated::*,
    tokens::*,
    traits::*,
};
pub use abi::StructMemoryKind;

use std::marker::PhantomData;

/// The main trait to go from untyped `SyntaxNode` to a typed ast. The conversion itself has zero
/// runtime cost; ast and syntax nodes have exactly the same representation; a pointer to the tree
/// root and a pointer to the node itself.
pub trait AstNode: Clone {
    fn can_cast(kind: SyntaxKind) -> bool;

    fn cast(syntax: SyntaxNode) -> Option<Self>
    where
        Self: Sized;
    fn syntax(&self) -> &SyntaxNode;
}

/// Like an `AstNode`, but wraps tokens rather than interior nodes.
pub trait AstToken {
    fn can_cast(kind: SyntaxKind) -> bool
    where
        Self: Sized;

    fn cast(token: SyntaxToken) -> Option<Self>
    where
        Self: Sized;

    fn syntax(&self) -> &SyntaxToken;

    fn text(&self) -> &str {
        self.syntax().text()
    }
}

/// An iterator over `SyntaxNode` children of a particular AST type.
#[derive(Debug)]
pub struct AstChildren<N> {
    inner: SyntaxNodeChildren,
    ph: PhantomData<N>,
}

impl<N> AstChildren<N> {
    fn new(parent: &SyntaxNode) -> Self {
        AstChildren {
            inner: parent.children(),
            ph: PhantomData,
        }
    }
}

impl<N: AstNode> Iterator for AstChildren<N> {
    type Item = N;
    fn next(&mut self) -> Option<N> {
        self.inner.by_ref().find_map(N::cast)
    }
}

fn child_opt<P: AstNode + ?Sized, C: AstNode>(parent: &P) -> Option<C> {
    children(parent).next()
}

fn children<P: AstNode + ?Sized, C: AstNode>(parent: &P) -> AstChildren<C> {
    AstChildren::new(parent.syntax())
}

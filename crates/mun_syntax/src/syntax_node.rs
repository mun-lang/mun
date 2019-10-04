//! This module defines Concrete Syntax Tree (CST), used by Mun.
//!
//! The CST includes comments and whitespace, provides a single node type,
//! `SyntaxNode`, and a basic traversal API (parent, children, siblings).
//!
//! The *real* implementation is in the (language-agnostic) `rowan` crate, this
//! modules just wraps its API.

use crate::{
    parsing::ParseError,
    syntax_error::{SyntaxError, SyntaxErrorKind},
    Parse, SmolStr, SyntaxKind, TextUnit,
};
use rowan::{GreenNodeBuilder, Language};

pub(crate) use rowan::GreenNode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MunLanguage {}
impl Language for MunLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::cursor::SyntaxKind) -> SyntaxKind {
        SyntaxKind::from(raw.0)
    }

    fn kind_to_raw(kind: SyntaxKind) -> rowan::cursor::SyntaxKind {
        rowan::cursor::SyntaxKind(kind.into())
    }
}

pub type SyntaxNode = rowan::SyntaxNode<MunLanguage>;
pub type SyntaxToken = rowan::SyntaxToken<MunLanguage>;
pub type SyntaxElement = rowan::NodeOrToken<SyntaxNode, SyntaxToken>;
pub type SyntaxNodeChildren = rowan::SyntaxNodeChildren<MunLanguage>;
pub type SyntaxElementChildren = rowan::SyntaxElementChildren<MunLanguage>;

pub use rowan::{Direction, NodeOrToken};

pub struct SyntaxTreeBuilder {
    errors: Vec<SyntaxError>,
    inner: GreenNodeBuilder,
}

impl Default for SyntaxTreeBuilder {
    fn default() -> SyntaxTreeBuilder {
        SyntaxTreeBuilder {
            errors: Vec::new(),
            inner: GreenNodeBuilder::new(),
        }
    }
}

impl SyntaxTreeBuilder {
    pub(crate) fn finish_raw(self) -> (GreenNode, Vec<SyntaxError>) {
        let green = self.inner.finish();
        (green, self.errors)
    }

    pub fn finish(self) -> Parse<SyntaxNode> {
        let (green, errors) = self.finish_raw();
        let node = SyntaxNode::new_root(green);
        //        if cfg!(debug_assertions) {
        //            crate::validation::validate_block_structure(&node);
        //        }
        Parse::new(node.green().clone(), errors)
    }

    pub fn token(&mut self, kind: SyntaxKind, text: SmolStr) {
        let kind = MunLanguage::kind_to_raw(kind);
        self.inner.token(kind, text)
    }

    pub fn start_node(&mut self, kind: SyntaxKind) {
        let kind = MunLanguage::kind_to_raw(kind);
        self.inner.start_node(kind)
    }

    pub fn finish_node(&mut self) {
        self.inner.finish_node()
    }

    pub fn error(&mut self, error: ParseError, text_pos: TextUnit) {
        let error = SyntaxError::new(SyntaxErrorKind::ParseError(error), text_pos);
        self.errors.push(error)
    }
}

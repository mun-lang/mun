use crate::{syntax_node::GreenNode, SyntaxError, SyntaxKind};

#[macro_use]
mod token_set;

mod event;
mod grammar;
pub mod lexer;
mod parser;
mod text_token_source;
mod text_tree_sink;

pub use lexer::tokenize;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParseError(pub String);

/// `TokenSource` abstract the source of the tokens.
trait TokenSource {
    fn current(&self) -> Token;

    /// Lookahead n token
    fn lookahead_nth(&self, n: usize) -> Token;

    /// bump cursor to next token
    fn bump(&mut self);

    /// Is the current token a specified keyword?
    fn is_keyword(&self, kw: &str) -> bool;
}

/// `TokenCursor` abstracts the cursor of `TokenSource` operates one.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Token {
    /// What is the current token?
    pub kind: SyntaxKind,

    /// Is the current token joined to the next one (`> >` vs `>>`).
    pub is_jointed_to_next: bool,
}

/// `TreeSink` abstracts details of a particular syntax tree implementation.
pub trait TreeSink {
    /// Adds new tokens to the current branch.
    fn token(&mut self, kind: SyntaxKind, n_tokens: u8);

    /// Starts new branch and make it current
    fn start_node(&mut self, kind: SyntaxKind);

    /// Finish current branch and restore previous branch as current.
    fn finish_node(&mut self);

    /// Note an error on the current branch
    fn error(&mut self, error: ParseError);
}

pub(crate) fn parse_text(text: &str) -> (GreenNode, Vec<SyntaxError>) {
    let tokens = tokenize(text);
    let mut token_source = text_token_source::TextTokenSource::new(text, &tokens);
    let mut tree_sink = text_tree_sink::TextTreeSink::new(text, &tokens);
    parse(&mut token_source, &mut tree_sink);
    tree_sink.finish()
}

fn parse_from_tokens<F>(token_source: &mut dyn TokenSource, tree_sink: &mut dyn TreeSink, f: F)
where
    F: FnOnce(&mut parser::Parser<'_>),
{
    let mut p = parser::Parser::new(token_source);
    f(&mut p);
    let events = p.finish();
    event::process(tree_sink, events);
}

/// Parse given tokens into the given sink as a rust file.
fn parse(token_source: &mut dyn TokenSource, tree_sink: &mut dyn TreeSink) {
    parse_from_tokens(token_source, tree_sink, grammar::root);
}

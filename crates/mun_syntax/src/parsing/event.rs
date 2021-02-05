///! It is intended to be completely decoupled from the parser, so as to allow to evolve the tree
/// representation and the parser algorithm independently.
///!
///! The `TreeSink` trait is the bridge between the parser and the tree builder: the parses
///! produces a stream of events like `start node`, `finish node` and `TreeSink` converts
///! this stream to a real tree.
use std::mem;

use crate::{
    parsing::{ParseError, TreeSink},
    SyntaxKind::{self, *},
};

/// `Parser` produces a flat list of `Events`'s. They are converted to a tree structure in a
/// separate pass via a `TreeSink`.
#[derive(Debug)]
pub(crate) enum Event {
    /// This event signifies the start of a node.
    /// It should be either abandoned (in which case the `kind` is `TOMBSTONE`, and the event is
    /// ignored), or completed via a `Finish` event.
    ///
    /// All tokens between a `Start` and a `Finish` become the children of the respective node.
    Start {
        kind: SyntaxKind,
        forward_parent: Option<u32>,
    },

    /// Completes the previous `Start` event
    Finish,

    /// Produce a single leaf-element.
    /// `n_raw_tokens` is used to glue complex contextual tokens.
    /// For example, the lexer tokenizes `>>` as `>`, `>`. `n_raw_tokens = 2` is used to produce
    /// a single `>>`.
    Token {
        kind: SyntaxKind,
        n_raw_tokens: u8,
    },

    Error {
        msg: ParseError,
    },
}

impl Event {
    pub(crate) fn tombstone() -> Self {
        Event::Start {
            kind: TOMBSTONE,
            forward_parent: None,
        }
    }
}

pub(super) fn process(sink: &mut dyn TreeSink, mut events: Vec<Event>) {
    let mut forward_parents = Vec::new();

    for i in 0..events.len() {
        match mem::replace(&mut events[i], Event::tombstone()) {
            Event::Start {
                kind: TOMBSTONE, ..
            } => (),
            Event::Start {
                kind,
                forward_parent,
            } => {
                forward_parents.push(kind);
                let mut idx = i;
                let mut fp = forward_parent;
                while let Some(fwd) = fp {
                    idx += fwd as usize;
                    fp = match mem::replace(&mut events[idx], Event::tombstone()) {
                        Event::Start {
                            kind,
                            forward_parent,
                        } => {
                            if kind != TOMBSTONE {
                                forward_parents.push(kind);
                            }
                            forward_parent
                        }
                        _ => unreachable!(),
                    }
                }

                for kind in forward_parents.drain(..).rev() {
                    sink.start_node(kind)
                }
            }
            Event::Finish => sink.finish_node(),
            Event::Token { kind, n_raw_tokens } => {
                sink.token(kind, n_raw_tokens);
            }
            Event::Error { msg } => sink.error(msg),
        }
    }
}

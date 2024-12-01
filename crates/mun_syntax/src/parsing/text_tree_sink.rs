use std::mem;

use crate::{
    parsing::{lexer::Token, ParseError, TreeSink},
    syntax_node::GreenNode,
    SyntaxError,
    SyntaxKind::{self, COMMENT, FUNCTION_DEF, WHITESPACE},
    SyntaxTreeBuilder, TextRange, TextSize,
};

pub(crate) struct TextTreeSink<'a> {
    text: &'a str,
    tokens: &'a [Token],
    text_pos: TextSize,
    token_pos: usize,
    state: State,
    inner: SyntaxTreeBuilder,
}

enum State {
    PendingStart,
    Normal,
    PendingFinish,
}

impl TreeSink for TextTreeSink<'_> {
    fn token(&mut self, kind: SyntaxKind, n_tokens: u8) {
        match mem::replace(&mut self.state, State::Normal) {
            State::PendingStart => unreachable!(),
            State::PendingFinish => self.inner.finish_node(),
            State::Normal => (),
        }
        self.eat_trivias();
        let n_tokens = n_tokens as usize;
        let len = self.tokens[self.token_pos..self.token_pos + n_tokens]
            .iter()
            .map(|it| it.len)
            .sum::<TextSize>();
        self.do_token(kind, len, n_tokens);
    }

    fn start_node(&mut self, kind: SyntaxKind) {
        match mem::replace(&mut self.state, State::Normal) {
            State::PendingStart => {
                self.inner.start_node(kind);
                // No need to attach trivias to previous node; there is no previous node.
                return;
            }
            State::PendingFinish => self.inner.finish_node(),
            State::Normal => (),
        }

        let n_trivias = self.tokens[self.token_pos..]
            .iter()
            .take_while(|it| it.kind.is_trivia())
            .count();
        let leading_trivias = &self.tokens[self.token_pos..self.token_pos + n_trivias];
        let mut trivia_end =
            self.text_pos + leading_trivias.iter().map(|it| it.len).sum::<TextSize>();

        let n_attached_trivias = {
            let leading_trivias = leading_trivias.iter().rev().map(|it| {
                let next_end = trivia_end - it.len;
                let range = TextRange::new(next_end, trivia_end);
                trivia_end = next_end;
                (it.kind, &self.text[range])
            });
            n_attached_trivias(kind, leading_trivias)
        };
        self.eat_n_trivias(n_trivias - n_attached_trivias);
        self.inner.start_node(kind);
        self.eat_n_trivias(n_attached_trivias);
    }

    fn finish_node(&mut self) {
        match mem::replace(&mut self.state, State::PendingFinish) {
            State::PendingStart => unreachable!(),
            State::PendingFinish => self.inner.finish_node(),
            State::Normal => (),
        }
    }

    fn error(&mut self, error: ParseError) {
        self.inner.error(error, self.text_pos);
    }
}

impl<'a> TextTreeSink<'a> {
    pub(super) fn new(text: &'a str, tokens: &'a [Token]) -> TextTreeSink<'a> {
        TextTreeSink {
            text,
            tokens,
            text_pos: 0.into(),
            token_pos: 0,
            state: State::PendingStart,
            inner: SyntaxTreeBuilder::default(),
        }
    }

    pub(super) fn finish(mut self) -> (GreenNode, Vec<SyntaxError>) {
        match mem::replace(&mut self.state, State::Normal) {
            State::PendingFinish => {
                self.eat_trivias();
                self.inner.finish_node();
            }
            State::PendingStart | State::Normal => unreachable!(),
        }

        self.inner.finish_raw()
    }

    fn eat_trivias(&mut self) {
        while let Some(&token) = self.tokens.get(self.token_pos) {
            if !token.kind.is_trivia() {
                break;
            }
            self.do_token(token.kind, token.len, 1);
        }
    }

    fn eat_n_trivias(&mut self, n: usize) {
        for _ in 0..n {
            let token = self.tokens[self.token_pos];
            assert!(token.kind.is_trivia());
            self.do_token(token.kind, token.len, 1);
        }
    }

    fn do_token(&mut self, kind: SyntaxKind, len: TextSize, n_tokens: usize) {
        let range = TextRange::at(self.text_pos, len);
        let text = &self.text[range];
        self.text_pos += len;
        self.token_pos += n_tokens;
        self.inner.token(kind, text);
    }
}

/// This method counts the number of preceding trivias that should be attached
/// to the to node of the given kind.
fn n_attached_trivias<'a>(
    kind: SyntaxKind,
    trivias: impl Iterator<Item = (SyntaxKind, &'a str)>,
) -> usize {
    match kind {
        FUNCTION_DEF => trivias
            .take_while(|(kind, text)| match kind {
                WHITESPACE => !text.contains("\n\n"),
                COMMENT => true,
                _ => unreachable!(),
            })
            .count(),
        _ => 0,
    }
}

use crate::{
    parsing::{lexer::Token, TokenSource},
    SyntaxKind::{self, *},
    TextUnit,
};

/// An implementation of `TokenSource` for text.
pub(crate) struct TextTokenSource {
    /// Holds the start position of each token
    start_offsets: Vec<TextUnit>,

    /// Non-whitespace/comment tokens
    tokens: Vec<Token>,
}

impl TokenSource for TextTokenSource {
    fn token_kind(&self, pos: usize) -> SyntaxKind {
        if pos >= self.tokens.len() {
            EOF
        } else {
            self.tokens[pos].kind
        }
    }

    fn is_token_joint_to_next(&self, pos: usize) -> bool {
        if (pos + 1) >= self.tokens.len() {
            true
        } else {
            self.start_offsets[pos] + self.tokens[pos].len == self.start_offsets[pos + 1]
        }
    }
}

impl TextTokenSource {
    /// Generate input for tokens (expect comment and whitespace).
    pub fn new(_text: &str, raw_tokens: &[Token]) -> Self {
        let mut tokens = Vec::new();
        let mut start_offsets = Vec::new();
        let mut len = 0.into();
        for &token in raw_tokens.iter() {
            if !token.kind.is_trivia() {
                tokens.push(token);
                start_offsets.push(len);
            }
            len += token.len;
        }

        TextTokenSource {
            start_offsets,
            tokens,
        }
    }
}

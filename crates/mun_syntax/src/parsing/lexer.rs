mod classes;
mod comments;
mod cursor;
mod numbers;
mod strings;

use self::{
    classes::{is_dec_digit, is_ident_continue, is_ident_start, is_whitespace},
    comments::scan_comment,
    cursor::Cursor,
    numbers::scan_number,
    strings::scan_string,
};
use crate::{
    SyntaxKind::{self, ERROR, IDENT, NEQ, STRING, UNDERSCORE, WHITESPACE},
    TextSize,
};

/// A token of Mun source
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Token {
    /// The kind of token
    pub kind: SyntaxKind,

    /// The length of the token
    pub len: TextSize,
}

/// Break a string up into its component tokens
pub fn tokenize(text: &str) -> Vec<Token> {
    let mut text = text;
    let mut result = Vec::new();
    while !text.is_empty() {
        let token = next_token(text);
        result.push(token);
        let len: u32 = token.len.into();
        text = &text[len as usize..];
    }
    result
}

/// Get the next token from a string
pub fn next_token(text: &str) -> Token {
    assert!(!text.is_empty());
    let mut ptr = Cursor::new(text);
    let c = ptr.bump().unwrap();
    let kind = next_token_inner(c, &mut ptr);
    let len = ptr.into_len();
    Token { kind, len }
}

fn next_token_inner(c: char, cursor: &mut Cursor<'_>) -> SyntaxKind {
    if is_whitespace(c) {
        cursor.bump_while(is_whitespace);
        return WHITESPACE;
    }

    if c == '/' {
        if let Some(kind) = scan_comment(cursor) {
            return kind;
        }
    }

    let ident_start = is_ident_start(c);
    if ident_start {
        return scan_identifier_or_keyword(c, cursor);
    }

    if is_dec_digit(c) {
        return scan_number(c, cursor);
    }

    if let Some(kind) = scan_index(c, cursor) {
        return kind;
    }

    if let Some(kind) = SyntaxKind::from_char(c) {
        return kind;
    }

    match c {
        '!' if cursor.matches('=') => {
            cursor.bump();
            return NEQ;
        }
        '"' | '\'' => {
            scan_string(c, cursor);
            return STRING;
        }
        _ => (),
    }
    ERROR
}

fn scan_identifier_or_keyword(c: char, cursor: &mut Cursor<'_>) -> SyntaxKind {
    match (c, cursor.current()) {
        ('_', None) => return UNDERSCORE,
        ('_', Some(c)) if !is_ident_continue(c) => return UNDERSCORE,
        _ => (),
    };
    cursor.bump_while(is_ident_continue);
    if let Some(kind) = SyntaxKind::from_keyword(cursor.current_token_text()) {
        return kind;
    }
    IDENT
}

fn scan_index(c: char, cursor: &mut Cursor<'_>) -> Option<SyntaxKind> {
    if c == '.' {
        let mut is_first = true;
        while let Some(cc) = cursor.current() {
            match cc {
                '0' => {
                    cursor.bump();
                    if is_first {
                        break;
                    }
                }
                '1'..='9' => {
                    cursor.bump();
                }
                _ => {
                    if is_first {
                        return None;
                    } else {
                        break;
                    }
                }
            }
            is_first = false;
        }
        Some(SyntaxKind::INDEX)
    } else {
        None
    }
}

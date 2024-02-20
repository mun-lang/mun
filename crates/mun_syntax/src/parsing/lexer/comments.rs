use crate::{
    parsing::lexer::cursor::Cursor,
    SyntaxKind::{self, COMMENT},
};

pub(crate) fn scan_comment(cursor: &mut Cursor<'_>) -> Option<SyntaxKind> {
    if cursor.matches('/') {
        bump_until_eol(cursor);
        Some(COMMENT)
    } else {
        scan_block_comment(cursor)
    }
}

fn scan_block_comment(cursor: &mut Cursor<'_>) -> Option<SyntaxKind> {
    if cursor.matches('*') {
        cursor.bump();
        let mut depth: u32 = 1;
        while depth > 0 {
            if cursor.matches_str("*/") {
                depth -= 1;
                cursor.bump();
                cursor.bump();
            } else if cursor.matches_str("/*") {
                depth += 1;
                cursor.bump();
                cursor.bump();
            } else if cursor.bump().is_none() {
                break;
            }
        }
        Some(COMMENT)
    } else {
        None
    }
}

fn bump_until_eol(cursor: &mut Cursor<'_>) {
    loop {
        if cursor.matches('\n') || cursor.matches_str("\r\n") {
            return;
        }
        if cursor.bump().is_none() {
            return;
        }
    }
}

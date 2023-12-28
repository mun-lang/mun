use crate::parsing::lexer::{
    classes::{is_ident_continue, is_ident_start},
    cursor::Cursor,
};

use crate::SyntaxKind::{self, FLOAT_NUMBER, INT_NUMBER};

pub(crate) fn scan_number(c: char, cursor: &mut Cursor<'_>) -> SyntaxKind {
    if c == '0' {
        match cursor.current().unwrap_or('\0') {
            'b' | 'o' => {
                cursor.bump();
                scan_digits(cursor, false);
            }
            'x' => {
                cursor.bump();
                scan_digits(cursor, true);
            }
            '0'..='9' | '_' | '.' | 'e' | 'E' => {
                scan_digits(cursor, false);
            }
            _ => return INT_NUMBER,
        }
    } else {
        scan_digits(cursor, false);
    }

    if cursor.matches('.')
        && !(cursor.matches_str("..") || cursor.matches_nth_if(1, is_ident_start))
    {
        cursor.bump();
        scan_digits(cursor, false);
        scan_float_exponent(cursor);
        scan_suffix(cursor);
        return FLOAT_NUMBER;
    }

    if cursor.matches('e') || cursor.matches('E') {
        scan_float_exponent(cursor);
        scan_suffix(cursor);
        return FLOAT_NUMBER;
    }

    scan_suffix(cursor);
    INT_NUMBER
}

fn scan_suffix(cursor: &mut Cursor<'_>) {
    if cursor.matches_nth_if(0, is_ident_start) {
        cursor.bump();
        cursor.bump_while(is_ident_continue);
    }
}

fn scan_digits(cursor: &mut Cursor<'_>, allow_hex: bool) {
    while let Some(c) = cursor.current() {
        match c {
            '_' | '0'..='9' => {
                cursor.bump();
            }
            'a'..='f' | 'A'..='F' if allow_hex => {
                cursor.bump();
            }
            _ => return,
        }
    }
}

fn scan_float_exponent(cursor: &mut Cursor<'_>) {
    if cursor.matches('e') || cursor.matches('E') {
        cursor.bump();
        if cursor.matches('-') || cursor.matches('+') {
            cursor.bump();
        }
        scan_digits(cursor, false);
    }
}

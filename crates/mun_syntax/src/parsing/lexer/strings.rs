use crate::parsing::lexer::cursor::Cursor;

pub(crate) fn scan_string(c: char, cursor: &mut Cursor) {
    let quote_type = c;
    while let Some(c) = cursor.current() {
        match c {
            '\\' => {
                cursor.bump();
                if cursor.matches('\\') || cursor.matches(quote_type) {
                    cursor.bump();
                }
            }
            c if c == quote_type => {
                cursor.bump();
                return;
            }
            _ => {
                cursor.bump();
            }
        }
    }
}

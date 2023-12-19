use unicode_xid::UnicodeXID;

pub fn is_whitespace(c: char) -> bool {
    c.is_whitespace()
}

pub fn is_ident_start(c: char) -> bool {
    c.is_ascii_lowercase()
        || c.is_ascii_uppercase()
        || c == '_'
        || (c > '\x7f' && UnicodeXID::is_xid_start(c))
}

pub fn is_ident_continue(c: char) -> bool {
    c.is_ascii_lowercase()
        || c.is_ascii_uppercase()
        || c.is_ascii_digit()
        || c == '_'
        || (c > '\x7f' && UnicodeXID::is_xid_continue(c))
}

pub fn is_dec_digit(c: char) -> bool {
    c.is_ascii_digit()
}

use unicode_xid::UnicodeXID;

pub fn is_whitespace(c: char) -> bool {
    c.is_whitespace()
}

pub fn is_ident_start(c: char) -> bool {
    ('a'..='z').contains(&c)
        || ('A'..='Z').contains(&c)
        || c == '_'
        || (c > '\x7f' && UnicodeXID::is_xid_start(c))
}

pub fn is_ident_continue(c: char) -> bool {
    ('a'..='z').contains(&c)
        || ('A'..='Z').contains(&c)
        || ('0'..='9').contains(&c)
        || c == '_'
        || (c > '\x7f' && UnicodeXID::is_xid_continue(c))
}

pub fn is_dec_digit(c: char) -> bool {
    ('0'..='9').contains(&c)
}

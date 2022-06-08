use crate::ast::{self, AstToken};
use std::{iter::Peekable, str::CharIndices};

impl ast::IntNumber {
    /// Returns a tuple containing the text part of the literal and an optional suffix. For example
    /// `1usize` will result in `("1", Some("usize"))`
    pub fn split_into_parts(&self) -> (&str, Option<&str>) {
        split_int_text_and_suffix(self.text())
    }
}

impl ast::FloatNumber {
    /// Returns a tuple containing the text part of the literal and an optional suffix. For example
    /// `1e5f32` will result in `("1e5", Some("f32"))`
    pub fn split_into_parts(&self) -> (&str, Option<&str>) {
        split_float_text_and_suffix(self.text())
    }
}

/// Given a string containing an integer literal (e.g `0x123` or `1234u32`), splits the string in the
/// value part and the suffix part.
fn split_int_text_and_suffix(text: &str) -> (&str, Option<&str>) {
    let base = match text.as_bytes() {
        [b'0', b'x', ..] => 16,
        [b'0', b'o', ..] => 8,
        [b'0', b'b', ..] => 2,
        _ => 10,
    };

    let mut iter = text.char_indices().peekable();

    // Skip base specifier
    if base != 10 {
        iter.next();
        iter.next();
    }

    // Skip digits in the string
    skip_digits(base, &mut iter);

    if let Some((idx, _)) = iter.next() {
        (&text[0..idx], Some(&text[idx..]))
    } else {
        (text, None)
    }
}

/// Skips all digits in the iterator that belong to the given base
fn skip_digits(base: usize, iter: &mut Peekable<CharIndices>) {
    while let Some((_, c)) = iter.peek() {
        if match c {
            '0'..='9' => true,
            'a'..='f' | 'A'..='F' if base > 10 => true,
            '_' => true,
            _ => false,
        } {
            iter.next();
        } else {
            break;
        }
    }
}

/// Given a string containing a float literal (e.g `123.4` or `1234.4f32`), splits the string in the
/// value part and the suffix part.
fn split_float_text_and_suffix(text: &str) -> (&str, Option<&str>) {
    let mut iter = text.char_indices().peekable();
    skip_digits(10, &mut iter);

    // Continue after a decimal seperator
    if let Some((_, '.')) = iter.peek() {
        iter.next();
        skip_digits(10, &mut iter);
    }

    // Continue after exponent
    if let Some((_, c)) = iter.peek() {
        if *c == 'e' || *c == 'E' {
            iter.next();

            if let Some((_, c)) = iter.peek() {
                if *c == '-' || *c == '+' {
                    iter.next();
                }
            }

            skip_digits(10, &mut iter);
        }
    }

    if let Some((idx, _)) = iter.next() {
        (&text[0..idx], Some(&text[idx..]))
    } else {
        (text, None)
    }
}

#[cfg(test)]
mod tests {
    use super::{split_float_text_and_suffix, split_int_text_and_suffix};

    #[test]
    fn split_int_and_suffix() {
        assert_eq!(split_int_text_and_suffix("123"), ("123", None));
        assert_eq!(split_int_text_and_suffix("0x123"), ("0x123", None));
        assert_eq!(split_int_text_and_suffix("123_456"), ("123_456", None));
        assert_eq!(split_int_text_and_suffix("0xfff32"), ("0xfff32", None));
        assert_eq!(split_int_text_and_suffix("0xff_f32"), ("0xff_f32", None));
        assert_eq!(
            split_int_text_and_suffix("0xff_u32"),
            ("0xff_", Some("u32"))
        );
        assert_eq!(
            split_int_text_and_suffix("0x0101u32"),
            ("0x0101", Some("u32"))
        );
        assert_eq!(
            split_int_text_and_suffix("0xffffu32"),
            ("0xffff", Some("u32"))
        );
        assert_eq!(
            split_int_text_and_suffix("0o71234u32"),
            ("0o71234", Some("u32"))
        );
    }

    #[test]
    fn split_float_and_suffix() {
        assert_eq!(split_float_text_and_suffix("123.0"), ("123.0", None));
        assert_eq!(
            split_float_text_and_suffix("123.0f32"),
            ("123.0", Some("f32"))
        );
        assert_eq!(
            split_float_text_and_suffix("123e10f32"),
            ("123e10", Some("f32"))
        );
        assert_eq!(
            split_float_text_and_suffix("123E10f32"),
            ("123E10", Some("f32"))
        );
        assert_eq!(
            split_float_text_and_suffix("123E+10f32"),
            ("123E+10", Some("f32"))
        );
        assert_eq!(
            split_float_text_and_suffix("123E-10f32"),
            ("123E-10", Some("f32"))
        );
        assert_eq!(
            split_float_text_and_suffix("123.123E10f32"),
            ("123.123E10", Some("f32"))
        );
    }
}

use super::{children, BinExpr};
use crate::ast::{child_opt, AstChildren, Literal};
use crate::{
    ast, AstNode, SmolStr,
    SyntaxKind::{self, *},
    SyntaxToken, TextRange, TextUnit,
};
use std::iter::Peekable;
use std::str::CharIndices;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PrefixOp {
    /// The `not` operator for logical inversion
    Not,
    /// The `-` operator for negation
    Neg,
}

impl ast::PrefixExpr {
    pub fn op_kind(&self) -> Option<PrefixOp> {
        match self.op_token()?.kind() {
            T![!] => Some(PrefixOp::Not),
            T![-] => Some(PrefixOp::Neg),
            _ => None,
        }
    }

    pub fn op_token(&self) -> Option<SyntaxToken> {
        self.syntax().first_child_or_token()?.into_token()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BinOp {
    Add,
    Subtract,
    Divide,
    Multiply,
    Remainder,
    //    Power,
    Assign,
    AddAssign,
    SubtractAssign,
    DivideAssign,
    MultiplyAssign,
    RemainderAssign,
    //    PowerAssign,
    Equals,
    NotEquals,
    LessEqual,
    Less,
    GreatEqual,
    Greater,
}

impl BinExpr {
    pub fn op_details(&self) -> Option<(SyntaxToken, BinOp)> {
        use SyntaxKind::*;
        self.syntax()
            .children_with_tokens()
            .filter_map(|it| it.into_token())
            .find_map(|c| match c.kind() {
                PLUS => Some((c, BinOp::Add)),
                MINUS => Some((c, BinOp::Subtract)),
                SLASH => Some((c, BinOp::Divide)),
                STAR => Some((c, BinOp::Multiply)),
                PERCENT => Some((c, BinOp::Remainder)),
                //                CARET => Some((c, BinOp::Power)),
                T![=] => Some((c, BinOp::Assign)),
                PLUSEQ => Some((c, BinOp::AddAssign)),
                MINUSEQ => Some((c, BinOp::SubtractAssign)),
                SLASHEQ => Some((c, BinOp::DivideAssign)),
                STAREQ => Some((c, BinOp::MultiplyAssign)),
                PERCENTEQ => Some((c, BinOp::RemainderAssign)),
                //                CARETEQ => Some((c, BinOp::PowerAssign)),
                EQEQ => Some((c, BinOp::Equals)),
                NEQ => Some((c, BinOp::NotEquals)),
                LT => Some((c, BinOp::Less)),
                LTEQ => Some((c, BinOp::LessEqual)),
                GT => Some((c, BinOp::Greater)),
                GTEQ => Some((c, BinOp::GreatEqual)),
                _ => None,
            })
    }

    pub fn op_kind(&self) -> Option<BinOp> {
        self.op_details().map(|t| t.1)
    }

    pub fn op_token(&self) -> Option<SyntaxToken> {
        self.op_details().map(|t| t.0)
    }

    pub fn lhs(&self) -> Option<ast::Expr> {
        children(self).next()
    }

    pub fn rhs(&self) -> Option<ast::Expr> {
        children(self).nth(1)
    }

    pub fn sub_exprs(&self) -> (Option<ast::Expr>, Option<ast::Expr>) {
        let mut children = children(self);
        let first = children.next();
        let second = children.next();
        (first, second)
    }
}

#[derive(PartialEq, Eq)]
pub enum FieldKind {
    Name(ast::NameRef),
    Index(SyntaxToken),
}

impl ast::FieldExpr {
    pub fn index_token(&self) -> Option<SyntaxToken> {
        self.syntax
            .children_with_tokens()
            .find(|e| e.kind() == SyntaxKind::INDEX)
            .and_then(|e| e.into_token())
    }

    pub fn field_access(&self) -> Option<FieldKind> {
        if let Some(nr) = self.name_ref() {
            Some(FieldKind::Name(nr))
        } else if let Some(tok) = self.index_token() {
            Some(FieldKind::Index(tok))
        } else {
            None
        }
    }

    pub fn field_range(&self) -> TextRange {
        let field_name = self.name_ref().map(|n| n.syntax().text_range());

        let field_index = self.index_token().map(|i| i.text_range());

        let start = field_name
            .map(|f| f.start())
            .or_else(|| field_index.map(|i| TextUnit::from_usize(i.start().to_usize() + 1)))
            .unwrap_or_else(|| self.syntax().text_range().start());

        let end = field_name
            .map(|f| f.end())
            .or_else(|| field_index.map(|f| f.end()))
            .unwrap_or_else(|| self.syntax().text_range().end());

        TextRange::from_to(start, end)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum LiteralKind {
    String,
    IntNumber,
    FloatNumber,
    Bool,
}

impl Literal {
    pub fn token(&self) -> SyntaxToken {
        self.syntax()
            .children_with_tokens()
            .find(|e| !e.kind().is_trivia())
            .and_then(|e| e.into_token())
            .unwrap()
    }

    pub fn kind(&self) -> LiteralKind {
        match self.token().kind() {
            STRING => LiteralKind::String,
            FLOAT_NUMBER => LiteralKind::FloatNumber,
            INT_NUMBER => LiteralKind::IntNumber,
            T![true] | T![false] => LiteralKind::Bool,
            _ => unreachable!(),
        }
    }

    pub fn text_and_suffix(&self) -> (SmolStr, Option<SmolStr>) {
        let token = self.token();
        let text = token.text();
        match self.kind() {
            LiteralKind::String => (text.clone(), None),
            LiteralKind::IntNumber => {
                let (str, suffix) = split_int_text_and_suffix(text);
                (SmolStr::new(str), suffix.map(SmolStr::new))
            }
            LiteralKind::FloatNumber => {
                let (str, suffix) = split_float_text_and_suffix(text);
                (SmolStr::new(str), suffix.map(SmolStr::new))
            }
            LiteralKind::Bool => (text.clone(), None),
        }
    }
}

/// Given a string containing an integer literal (e.g `0x123` or `1234u32`), split the string in the
/// value part and the suffix part.
fn split_int_text_and_suffix(text: &str) -> (&str, Option<&str>) {
    let base = match text.as_bytes() {
        [b'0', b'x', ..] => 16,
        [b'0', b'b', ..] => 8,
        [b'0', b'o', ..] => 2,
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

/// Skip all digits in the iterator that belong to the given base
fn skip_digits(base: usize, iter: &mut Peekable<CharIndices>) {
    while let Some((_, c)) = iter.peek() {
        if match c {
            '0'..='1' => true,
            '2'..='8' if base > 2 => true,
            '9' if base > 8 => true,
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

/// Given a string containing an float literal (e.g `123.4` or `1234.4f32`), split the string in the
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElseBranch {
    Block(ast::BlockExpr),
    IfExpr(ast::IfExpr),
}

impl ast::IfExpr {
    pub fn then_branch(&self) -> Option<ast::BlockExpr> {
        self.blocks().next()
    }
    pub fn else_branch(&self) -> Option<ElseBranch> {
        let res = match self.blocks().nth(1) {
            Some(block) => ElseBranch::Block(block),
            None => {
                let elif: ast::IfExpr = child_opt(self)?;
                ElseBranch::IfExpr(elif)
            }
        };
        Some(res)
    }

    fn blocks(&self) -> AstChildren<ast::BlockExpr> {
        children(self)
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

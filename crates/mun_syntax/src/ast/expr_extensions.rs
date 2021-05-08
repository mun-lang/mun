use super::{children, BinExpr};
use crate::ast::{child_opt, AstChildren, Literal};
use crate::{
    ast, AstNode, SmolStr,
    SyntaxKind::{self, *},
    SyntaxToken, TextRange, TextSize,
};
use std::iter::Peekable;
use std::ops::Add;
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
    /// The `+` operator for addition
    Add,
    /// The `-` operator for subtraction
    Subtract,
    /// The `*` operator for multiplication
    Multiply,
    /// The `/` operator for division
    Divide,
    /// The `%` operator for remainder after division
    Remainder,
    /// The `&&` operator for boolean AND
    BooleanAnd,
    /// The `||` operator for boolean OR
    BooleanOr,
    /// The `<<` operator for left shift
    LeftShift,
    /// The `>>` operator for right shift
    RightShift,
    /// The `|` operator for bitwise OR
    BitwiseOr,
    /// The `&` operator for bitwise AND
    BitwiseAnd,
    /// The `^` operator for bitwise XOR
    BitwiseXor,
    /// The `=` operator for assignment
    Assign,
    /// The `+=` operator for assignment after addition
    AddAssign,
    /// The `-=` operator for assignment after subtraction
    SubtractAssign,
    /// The `*=` operator for assignment after multiplication
    MultiplyAssign,
    /// The `/=` operator for assignment after division
    DivideAssign,
    /// The `%=` operator for assignment after remainders
    RemainderAssign,
    /// The `<<=` operator for assignment after shifting left
    LeftShiftAssign,
    /// The `>>=` operator for assignment after shifting right
    RightShiftAssign,
    /// The `&=` operator for assignment after bitwise AND
    BitAndAssign,
    /// The `|=` operator for assignment after bitwise OR
    BitOrAssign,
    /// The `^=` operator for assignment after bitwise XOR
    BitXorAssign,
    /// The `==` operator for equality testing
    Equals,
    /// The `!=` operator for inequality testing
    NotEqual,
    /// The `<=` operator for lesser-equal testing
    LessEqual,
    /// The `<` operator for comparison
    Less,
    /// The `>=` operator for greater-equal testing
    GreatEqual,
    /// The `>` operator for comparison
    Greater,
}

impl BinExpr {
    pub fn op_details(&self) -> Option<(SyntaxToken, BinOp)> {
        self.syntax()
            .children_with_tokens()
            .filter_map(|it| it.into_token())
            .find_map(|c| {
                let bin_op = match c.kind() {
                    T![+] => BinOp::Add,
                    T![-] => BinOp::Subtract,
                    T![*] => BinOp::Multiply,
                    T![/] => BinOp::Divide,
                    T![%] => BinOp::Remainder,
                    T![<<] => BinOp::LeftShift,
                    T![>>] => BinOp::RightShift,
                    T![^] => BinOp::BitwiseXor,
                    T![|] => BinOp::BitwiseOr,
                    T![&] => BinOp::BitwiseAnd,
                    T![=] => BinOp::Assign,
                    T![+=] => BinOp::AddAssign,
                    T![-=] => BinOp::SubtractAssign,
                    T![/=] => BinOp::DivideAssign,
                    T![*=] => BinOp::MultiplyAssign,
                    T![%=] => BinOp::RemainderAssign,
                    T![<<=] => BinOp::LeftShiftAssign,
                    T![>>=] => BinOp::RightShiftAssign,
                    T![&=] => BinOp::BitAndAssign,
                    T![|=] => BinOp::BitOrAssign,
                    T![^=] => BinOp::BitXorAssign,
                    T![==] => BinOp::Equals,
                    T![!=] => BinOp::NotEqual,
                    T![<] => BinOp::Less,
                    T![<=] => BinOp::LessEqual,
                    T![>] => BinOp::Greater,
                    T![>=] => BinOp::GreatEqual,
                    T![&&] => BinOp::BooleanAnd,
                    T![||] => BinOp::BooleanOr,
                    _ => return None,
                };
                Some((c, bin_op))
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
        } else {
            self.index_token().map(FieldKind::Index)
        }
    }

    pub fn field_range(&self) -> TextRange {
        let field_name = self.name_ref().map(|n| n.syntax().text_range());

        let field_index = self.index_token().map(|i| i.text_range());

        let start = field_name
            .map(|f| f.start())
            .or_else(|| field_index.map(|i| i.start().add(TextSize::from(1u32))))
            .unwrap_or_else(|| self.syntax().text_range().start());

        let end = field_name
            .map(|f| f.end())
            .or_else(|| field_index.map(|f| f.end()))
            .unwrap_or_else(|| self.syntax().text_range().end());

        TextRange::new(start, end)
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

    /// Returns a tuple containing the text part of the literal and an optional suffix. For example
    /// `1e5f32` will result in `("1e5", Some("f32"))`
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

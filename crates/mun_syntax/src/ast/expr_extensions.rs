use std::ops::Add;

use crate::{
    ast::{self, child_opt, children, AstChildren, AstToken, BinExpr, Literal},
    AstNode, SyntaxKind, SyntaxToken, TextRange, TextSize,
};

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
            .filter_map(rowan::NodeOrToken::into_token)
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
            .and_then(rowan::NodeOrToken::into_token)
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
            .map(rowan::TextRange::start)
            .or_else(|| field_index.map(|i| i.start().add(TextSize::from(1u32))))
            .unwrap_or_else(|| self.syntax().text_range().start());

        let end = field_name
            .map(rowan::TextRange::end)
            .or_else(|| field_index.map(rowan::TextRange::end))
            .unwrap_or_else(|| self.syntax().text_range().end());

        TextRange::new(start, end)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum LiteralKind {
    String(ast::String),
    IntNumber(ast::IntNumber),
    FloatNumber(ast::FloatNumber),
    Bool(bool),
}

impl Literal {
    pub fn token(&self) -> SyntaxToken {
        self.syntax()
            .children_with_tokens()
            .find(|e| !e.kind().is_trivia())
            .and_then(rowan::NodeOrToken::into_token)
            .unwrap()
    }

    pub fn kind(&self) -> LiteralKind {
        let token = self.token();

        if let Some(t) = ast::IntNumber::cast(token.clone()) {
            return LiteralKind::IntNumber(t);
        } else if let Some(t) = ast::FloatNumber::cast(token.clone()) {
            return LiteralKind::FloatNumber(t);
        } else if let Some(t) = ast::String::cast(token.clone()) {
            return LiteralKind::String(t);
        }

        match token.kind() {
            T![true] => LiteralKind::Bool(true),
            T![false] => LiteralKind::Bool(false),
            _ => unreachable!(),
        }
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
        let res = if let Some(block) = self.blocks().nth(1) {
            ElseBranch::Block(block)
        } else {
            let elif: ast::IfExpr = child_opt(self)?;
            ElseBranch::IfExpr(elif)
        };
        Some(res)
    }

    fn blocks(&self) -> AstChildren<ast::BlockExpr> {
        children(self)
    }
}

impl ast::IndexExpr {
    pub fn base(&self) -> Option<ast::Expr> {
        children(self).next()
    }
    pub fn index(&self) -> Option<ast::Expr> {
        children(self).nth(1)
    }
}

use super::{children, BinExpr};
use crate::ast::{Literal, child_opt, AstChildren};
use crate::{
    ast, AstNode,
    SyntaxKind::{self, *},
    SyntaxToken,
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
    Add,
    Subtract,
    Divide,
    Multiply,
    //    Remainder,
    //    Power,
    Assign,
    AddAssign,
    SubtractAssign,
    DivideAssign,
    MultiplyAssign,
    //    RemainderAssign,
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
                //                PERCENT => Some((c, BinOp::Remainder)),
                //                CARET => Some((c, BinOp::Power)),
                T![=] => Some((c, BinOp::Assign)),
                PLUSEQ => Some((c, BinOp::AddAssign)),
                MINUSEQ => Some((c, BinOp::SubtractAssign)),
                SLASHEQ => Some((c, BinOp::DivideAssign)),
                STAREQ => Some((c, BinOp::MultiplyAssign)),
                //                PERCENTEQ => Some((c, BinOp::RemainderAssign)),
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
        children(self).nth(0)
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElseBranch {
    Block(ast::BlockExpr),
    IfExpr(ast::IfExpr),
}

impl ast::IfExpr {
    pub fn then_branch(&self) -> Option<ast::BlockExpr> {
        self.blocks().nth(0)
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

use crate::{
    ast::{self, child_opt, AstNode, NameOwner},
    T,
};
use crate::{SmolStr, SyntaxNode};
use text_unit::TextRange;

impl ast::Name {
    pub fn text(&self) -> &SmolStr {
        text_of_first_token(self.syntax())
    }
}

impl ast::NameRef {
    pub fn text(&self) -> &SmolStr {
        text_of_first_token(self.syntax())
    }
}

impl ast::FunctionDef {
    pub fn signature_range(&self) -> TextRange {
        let fn_kw = self
            .syntax()
            .children_with_tokens()
            .find(|p| p.kind() == T![fn])
            .map(|kw| kw.text_range());
        let name = self.name().map(|n| n.syntax.text_range());
        let param_list = self.param_list().map(|p| p.syntax.text_range());
        let ret_type = self.ret_type().map(|r| r.syntax.text_range());

        let start = fn_kw
            .map(|kw| kw.start())
            .unwrap_or_else(|| self.syntax.text_range().start());

        let end = ret_type
            .map(|p| p.end())
            .or_else(|| param_list.map(|name| name.end()))
            .or_else(|| name.map(|name| name.end()))
            .or_else(|| fn_kw.map(|kw| kw.end()))
            .unwrap_or_else(|| self.syntax().text_range().end());

        TextRange::from_to(start, end)
    }
}

fn text_of_first_token(node: &SyntaxNode) -> &SmolStr {
    node.green()
        .children()
        .first()
        .and_then(|it| it.as_token())
        .unwrap()
        .text()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSegmentKind {
    Name(ast::NameRef),
    SelfKw,
    SuperKw,
}

impl ast::PathSegment {
    pub fn parent_path(&self) -> ast::Path {
        self.syntax()
            .parent()
            .and_then(ast::Path::cast)
            .expect("segments are always nested in paths")
    }

    pub fn kind(&self) -> Option<PathSegmentKind> {
        let res = if let Some(name_ref) = self.name_ref() {
            PathSegmentKind::Name(name_ref)
        } else {
            match self.syntax().first_child_or_token()?.kind() {
                T![self] => PathSegmentKind::SelfKw,
                T![super] => PathSegmentKind::SuperKw,
                _ => return None,
            }
        };
        Some(res)
    }

    pub fn has_colon_colon(&self) -> bool {
        match self.syntax.first_child_or_token().map(|s| s.kind()) {
            Some(T![::]) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructKind {
    Record(ast::RecordFieldDefList),
    Unit,
}

impl StructKind {
    fn from_node<N: AstNode>(node: &N) -> StructKind {
        if let Some(r) = child_opt::<_, ast::RecordFieldDefList>(node) {
            StructKind::Record(r)
        } else {
            StructKind::Unit
        }
    }
}

impl ast::StructDef {
    pub fn kind(&self) -> StructKind {
        StructKind::from_node(self)
    }
}

use crate::{
    ast::{self, child_opt, AstNode, NameOwner},
    SyntaxKind, T,
};
use crate::{SmolStr, SyntaxNode};
use abi::StructMemoryKind;
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

    pub fn as_tuple_field(&self) -> Option<usize> {
        self.syntax().children_with_tokens().find_map(|c| {
            if c.kind() == SyntaxKind::INT_NUMBER {
                c.as_token()
                    .and_then(|tok| tok.text().as_str().parse().ok())
            } else {
                None
            }
        })
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
    PackageKw,
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
                T![package] => PathSegmentKind::PackageKw,
                _ => return None,
            }
        };
        Some(res)
    }

    pub fn has_colon_colon(&self) -> bool {
        matches!(
            self.syntax.first_child_or_token().map(|s| s.kind()),
            Some(T![::])
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructKind {
    Record(ast::RecordFieldDefList),
    Tuple(ast::TupleFieldDefList),
    Unit,
}

impl StructKind {
    fn from_node<N: AstNode>(node: &N) -> StructKind {
        if let Some(r) = child_opt::<_, ast::RecordFieldDefList>(node) {
            StructKind::Record(r)
        } else if let Some(t) = child_opt::<_, ast::TupleFieldDefList>(node) {
            StructKind::Tuple(t)
        } else {
            StructKind::Unit
        }
    }
}

impl ast::MemoryTypeSpecifier {
    pub fn kind(&self) -> StructMemoryKind {
        if self.is_value() {
            StructMemoryKind::Value
        } else if self.is_gc() {
            StructMemoryKind::GC
        } else {
            StructMemoryKind::default()
        }
    }

    fn is_gc(&self) -> bool {
        self.syntax()
            .children_with_tokens()
            .any(|it| it.kind() == SyntaxKind::GC_KW)
    }

    fn is_value(&self) -> bool {
        self.syntax()
            .children_with_tokens()
            .any(|it| it.kind() == SyntaxKind::VALUE_KW)
    }
}

impl ast::StructDef {
    pub fn kind(&self) -> StructKind {
        StructKind::from_node(self)
    }
    pub fn signature_range(&self) -> TextRange {
        let struct_kw = self
            .syntax()
            .children_with_tokens()
            .find(|p| p.kind() == T![struct])
            .map(|kw| kw.text_range());
        let name = self.name().map(|n| n.syntax.text_range());

        let start = struct_kw
            .map(|kw| kw.start())
            .unwrap_or_else(|| self.syntax.text_range().start());

        let end = name
            .map(|name| name.end())
            .or_else(|| struct_kw.map(|kw| kw.end()))
            .unwrap_or_else(|| self.syntax().text_range().end());

        TextRange::from_to(start, end)
    }
}

pub enum VisibilityKind {
    PubPackage,
    PubSuper,
    Pub,
}

impl ast::Visibility {
    pub fn kind(&self) -> VisibilityKind {
        if self.is_pub_package() {
            VisibilityKind::PubPackage
        } else if self.is_pub_super() {
            VisibilityKind::PubSuper
        } else {
            VisibilityKind::Pub
        }
    }

    fn is_pub_package(&self) -> bool {
        self.syntax()
            .children_with_tokens()
            .any(|it| it.kind() == T![package])
    }

    fn is_pub_super(&self) -> bool {
        self.syntax()
            .children_with_tokens()
            .any(|it| it.kind() == T![super])
    }
}

impl ast::UseTree {
    pub fn has_star_token(&self) -> bool {
        self.syntax()
            .children_with_tokens()
            .any(|it| it.kind() == T![*])
    }
}

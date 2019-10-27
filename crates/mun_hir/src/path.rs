use crate::{AsName, Name};
use mun_syntax::ast;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path {
    pub kind: PathKind,
    pub segments: Vec<PathSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PathSegment {
    pub name: Name,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathKind {
    Plain,
    Self_,
    Super,
    Abs,
}

impl Path {
    /// Converts an `ast::Path` to `Path`.
    pub fn from_ast(path: ast::Path) -> Option<Path> {
        let mut kind = PathKind::Plain;
        let mut segments = Vec::new();
        //        loop {
        let segment = path.segment()?;

        if segment.has_colon_colon() {
            kind = PathKind::Abs;
        }

        match segment.kind()? {
            ast::PathSegmentKind::Name(name) => {
                let segment = PathSegment {
                    name: name.as_name(),
                };
                segments.push(segment);
            }
            ast::PathSegmentKind::SelfKw => {
                kind = PathKind::Self_;
                //                    break;
            }
            ast::PathSegmentKind::SuperKw => {
                kind = PathKind::Super;
                //                    break;
            }
        }
        //            break;
        //        }
        segments.reverse();
        Some(Path { kind, segments })
    }

    /// Converts an `ast::NameRef` into a single-identifier `Path`.
    pub fn from_name_ref(name_ref: &ast::NameRef) -> Path {
        name_ref.as_name().into()
    }

    /// `true` if this path is a single identifier, like `bar`
    pub fn is_ident(&self) -> bool {
        self.kind == PathKind::Plain && self.segments.len() == 1
    }

    /// If this path represents a single identifier, like `foo`, return its name.
    pub fn as_ident(&self) -> Option<&Name> {
        if self.is_ident() {
            return self.segments.first().map(|s| &s.name);
        }
        None
    }
}

impl From<Name> for Path {
    fn from(name: Name) -> Path {
        Path {
            kind: PathKind::Plain,
            segments: vec![PathSegment { name }],
        }
    }
}

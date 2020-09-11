use crate::{AsName, Name};
use mun_syntax::ast;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path {
    pub kind: PathKind,
    pub segments: Vec<Name>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathKind {
    Plain,
    // `self` is Super(0)
    Super(u8),
    Package,
}

impl Path {
    /// Converts an `ast::Path` to `Path`.
    pub fn from_ast(mut path: ast::Path) -> Option<Path> {
        let mut kind = PathKind::Plain;
        let mut segments = Vec::new();
        loop {
            let segment = path.segment()?;

            match segment.kind()? {
                ast::PathSegmentKind::Name(name) => {
                    segments.push(name.as_name());
                }
                ast::PathSegmentKind::SelfKw => {
                    kind = PathKind::Super(0);
                    break;
                }
                ast::PathSegmentKind::SuperKw => {
                    kind = PathKind::Super(1);
                    break;
                }
                ast::PathSegmentKind::PackageKw => {
                    kind = PathKind::Package;
                    break;
                }
            }

            path = match path.qualifier() {
                Some(p) => p,
                None => break,
            }
        }
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
            return self.segments.first();
        }
        None
    }
}

impl From<Name> for Path {
    fn from(name: Name) -> Path {
        Path {
            kind: PathKind::Plain,
            segments: vec![name],
        }
    }
}

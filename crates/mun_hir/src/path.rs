use crate::{AsName, Name};
use mun_syntax::ast;
use mun_syntax::ast::{NameOwner, PathSegmentKind};

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

/// A possible import alias e.g. `Foo as Bar` or `Foo as _`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportAlias {
    /// Unnamed alias, as in `use Foo as _;`
    Underscore,
    /// Named alias
    Alias(Name),
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

    /// Constructs a path from its segments.
    pub fn from_segments(kind: PathKind, segments: impl IntoIterator<Item = Name>) -> Path {
        let segments = segments.into_iter().collect::<Vec<_>>();
        Path { kind, segments }
    }

    /// Calls `cb` with all paths, represented by this use item. For the use statement:
    /// ```mun
    /// use foo::{self, Bar};
    /// ```
    /// the function will call the callback twice. Once for `foo` and once for `foo::Bar`.
    pub(crate) fn expand_use_item(
        item_src: &ast::Use,
        mut cb: impl FnMut(Path, &ast::UseTree, /* is_glob */ bool, Option<ImportAlias>),
    ) {
        if let Some(tree) = item_src.use_tree() {
            lower_use_tree(None, &tree, &mut cb);
        }
    }
}

/// Given an `ast::UseTree` and an optional prefix, call a callback function for every item that is
/// contained in the import tree.
///
/// For the use statement:
/// ```mun
/// use foo::{self, Bar};
/// ```
/// the function will call the callback twice. Once for `foo` and once for `foo::Bar`.
fn lower_use_tree(
    prefix: Option<Path>,
    tree: &ast::UseTree,
    cb: &mut impl FnMut(Path, &ast::UseTree, bool, Option<ImportAlias>),
) {
    if let Some(use_tree_list) = tree.use_tree_list() {
        let prefix = match tree.path() {
            None => prefix,
            Some(path) => convert_path(prefix, &path),
        };
        for child_tree in use_tree_list.use_trees() {
            lower_use_tree(prefix.clone(), &child_tree, cb);
        }
    } else {
        let alias = tree.rename().map(|a| {
            a.name()
                .map(|it| it.as_name())
                .map_or(ImportAlias::Underscore, ImportAlias::Alias)
        });

        let is_glob = tree.has_star_token();
        if let Some(ast_path) = tree.path() {
            // Handle self in a path.
            if ast_path.qualifier().is_none() {
                if let Some(segment) = ast_path.segment() {
                    if segment.kind() == Some(ast::PathSegmentKind::SelfKw) {
                        if let Some(prefix) = prefix {
                            cb(prefix, tree, false, alias);
                            return;
                        }
                    }
                }
            }
            if let Some(path) = convert_path(prefix, &ast_path) {
                cb(path, tree, is_glob, alias);
            }
        } else if is_glob {
            if let Some(prefix) = prefix {
                cb(prefix, tree, is_glob, None);
            }
        }
    }
}

/// Constructs a `mun_hir::Path` from an `ast::Path` and an optional prefix.
fn convert_path(prefix: Option<Path>, path: &ast::Path) -> Option<Path> {
    let prefix = if let Some(qualifier) = path.qualifier() {
        Some(convert_path(prefix, &qualifier)?)
    } else {
        prefix
    };

    let segment = path.segment()?;
    let res = match segment.kind()? {
        ast::PathSegmentKind::Name(name_ref) => {
            let mut res = prefix.unwrap_or_else(|| Path {
                kind: PathKind::Plain,
                segments: Vec::with_capacity(1),
            });
            res.segments.push(name_ref.as_name());
            res
        }
        ast::PathSegmentKind::PackageKw => {
            if prefix.is_some() {
                return None;
            }
            Path::from_segments(PathKind::Package, std::iter::empty())
        }
        PathSegmentKind::SelfKw => {
            if prefix.is_some() {
                return None;
            }
            Path::from_segments(PathKind::Super(0), std::iter::empty())
        }
        PathSegmentKind::SuperKw => {
            if prefix.is_some() {
                return None;
            }
            Path::from_segments(PathKind::Super(1), std::iter::empty())
        }
    };
    Some(res)
}

impl From<Name> for Path {
    fn from(name: Name) -> Path {
        Path {
            kind: PathKind::Plain,
            segments: vec![name],
        }
    }
}

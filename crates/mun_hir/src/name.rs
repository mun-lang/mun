use mun_syntax::{ast, SmolStr};
use std::fmt;

/// `Name` is a wrapper around string, which is used in hir for both references
/// and declarations.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Name {
    text: SmolStr,
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.text, f)
    }
}

impl fmt::Debug for Name {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.text, f)
    }
}

impl Name {
    const fn new(text: SmolStr) -> Name {
        Name { text }
    }

    pub(crate) fn missing() -> Name {
        Name::new("[missing name]".into())
    }
}

pub(crate) trait AsName {
    fn as_name(&self) -> Name;
}

impl AsName for ast::NameRef {
    fn as_name(&self) -> Name {
        Name::new(self.text().clone())
    }
}

impl AsName for ast::Name {
    fn as_name(&self) -> Name {
        Name::new(self.text().clone())
    }
}

pub(crate) const FLOAT: Name = Name::new(SmolStr::new_inline_from_ascii(5, b"float"));
pub(crate) const INT: Name = Name::new(SmolStr::new_inline_from_ascii(3, b"int"));
pub(crate) const BOOLEAN: Name = Name::new(SmolStr::new_inline_from_ascii(4, b"bool"));

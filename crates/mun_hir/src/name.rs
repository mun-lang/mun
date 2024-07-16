use std::fmt;

use mun_syntax::{ast, SmolStr};

/// `Name` is a wrapper around string, which is used in `mun_hir` for both
/// references and declarations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Name(Repr);

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Repr {
    Text(SmolStr),
    TupleField(usize),
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Repr::Text(text) => fmt::Display::fmt(&text, f),
            Repr::TupleField(idx) => fmt::Display::fmt(&idx, f),
        }
    }
}

impl Name {
    /// Note: this is private to make creating name from random string hard.
    /// Hopefully, this should allow us to integrate hygiene cleaner in the
    /// future, and to switch to interned representation of names.
    const fn new_text(text: SmolStr) -> Name {
        Name(Repr::Text(text))
    }

    pub(crate) fn new_tuple_field(idx: usize) -> Name {
        Name(Repr::TupleField(idx))
    }

    /// Shortcut to create inline plain text name
    const fn new_static(text: &'static str) -> Name {
        Name::new_text(SmolStr::new_static(text))
    }

    /// Resolve a name from the text of token.
    fn resolve(raw_text: &str) -> Name {
        match raw_text.strip_prefix("r#") {
            Some(text) => Name::new_text(SmolStr::new(text)),
            None => Name::new_text(raw_text.into()),
        }
    }

    pub(crate) fn new(text: impl AsRef<str>) -> Name {
        Name::new_text(SmolStr::new(text))
    }

    pub(crate) fn missing() -> Name {
        Name::new_text("[missing name]".into())
    }

    pub(crate) fn as_tuple_index(&self) -> Option<usize> {
        match self.0 {
            Repr::TupleField(idx) => Some(idx),
            Repr::Text(_) => None,
        }
    }

    /// Returns the text this name represents if it isn't a tuple field.
    pub fn as_str(&self) -> Option<&str> {
        match &self.0 {
            Repr::Text(it) => Some(it),
            Repr::TupleField(_) => None,
        }
    }
}

pub(crate) trait AsName {
    fn as_name(&self) -> Name;
}

impl AsName for ast::NameRef {
    fn as_name(&self) -> Name {
        match self.as_tuple_field() {
            Some(idx) => Name::new_tuple_field(idx),
            None => Name::resolve(&self.text()),
        }
    }
}

impl AsName for ast::Name {
    fn as_name(&self) -> Name {
        Name::resolve(&self.text())
    }
}

impl AsName for ast::FieldKind {
    fn as_name(&self) -> Name {
        match self {
            ast::FieldKind::Name(nr) => nr.as_name(),
            ast::FieldKind::Index(idx) => Name::new_tuple_field(idx.text()[1..].parse().unwrap()),
        }
    }
}

pub mod known {
    macro_rules! known_names {
        ($($ident:ident),* $(,)?) => {
            $(
                #[allow(bad_style)]
                pub const $ident: super::Name =
                    super::Name::new_static(stringify!($ident));
            )*
        };
    }

    known_names!(
        // Primitives
        int, isize, i8, i16, i32, i64, i128, uint, usize, u8, u16, u32, u64, u128, float, f32, f64,
        bool,
    );

    // self/Self cannot be used as an identifier
    pub const SELF_PARAM: super::Name = super::Name::new_static("self");
    pub const SELF_TYPE: super::Name = super::Name::new_static("Self");

    #[macro_export]
    macro_rules! name {
        (self) => {
            $crate::name::known::SELF_PARAM
        };
        (Self) => {
            $crate::name::known::SELF_TYPE
        };
        ($ident:ident) => {
            $crate::name::known::$ident
        };
    }
}

pub use crate::name;

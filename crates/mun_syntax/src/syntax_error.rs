use crate::parsing::ParseError;
use mun_errors::Location;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SyntaxError {
    kind: SyntaxErrorKind,
    location: Location,
}

impl SyntaxError {
    pub fn new<L: Into<Location>>(kind: SyntaxErrorKind, loc: L) -> SyntaxError {
        SyntaxError {
            kind,
            location: loc.into(),
        }
    }

    pub fn kind(&self) -> SyntaxErrorKind {
        self.kind.clone()
    }

    pub fn location(&self) -> Location {
        self.location.clone()
    }
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.kind.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SyntaxErrorKind {
    ParseError(ParseError),
}

impl fmt::Display for SyntaxErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::SyntaxErrorKind::*;
        match self {
            ParseError(msg) => write!(f, "{}", msg.0),
        }
    }
}

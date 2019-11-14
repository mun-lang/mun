mod location;

pub use crate::location::Location;

/// Defines the severity of a diagnostics.
/// TODO: Contains only Error, for now, maybe add some more?
#[derive(Clone, Copy, Debug, PartialEq, Hash)]
pub enum Level {
    Error,
}

#[derive(Clone, Debug, PartialEq, Hash)]
pub struct Diagnostic {
    pub level: Level,
    pub loc: location::Location,
    pub message: String,
}

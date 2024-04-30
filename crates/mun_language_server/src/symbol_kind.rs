/// Defines a set of symbols that can live in a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SymbolKind {
    Field,
    Function,
    Local,
    Module,
    SelfParam,
    SelfType,
    Struct,
    TypeAlias,
}

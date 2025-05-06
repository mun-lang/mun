/// Defines a set of symbols that can live in a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SymbolKind {
    Const,
    Field,
    Function,
    Method,
    Local,
    Module,
    Impl,
    SelfParam,
    SelfType,
    Struct,
    TypeAlias,
}

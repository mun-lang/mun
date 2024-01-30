use crate::SymbolKind;

/// A `CompletionItem` describes a single completion variant in an editor.
#[derive(Clone, Debug)]
pub struct CompletionItem {
    /// Used for tests to filter certain type of completions
    #[allow(unused)]
    pub completion_kind: CompletionKind,

    /// Label in the completion pop up which identifies completion.
    pub label: String,

    /// The type of completion
    pub kind: Option<CompletionItemKind>,

    /// Additional info to show in the UI pop up.
    pub detail: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum CompletionKind {
    /// Your usual "complete all valid identifiers".
    Reference,
    BuiltinType,
}

/// Type of completion used to provide hints to the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(unused)]
pub enum CompletionItemKind {
    SymbolKind(SymbolKind),
    Attribute,
    Binding,
    BuiltinType,
    Keyword,
    Method,
    Snippet,
    UnresolvedReference,
}

impl CompletionItemKind {
    /// Returns a tag that describes the type of item that was completed. This
    /// is only used in tests, to be able to distinguish between items with
    /// the same name.
    #[cfg(test)]
    pub(crate) fn tag(&self) -> &'static str {
        match self {
            CompletionItemKind::SymbolKind(kind) => match kind {
                SymbolKind::Field => "fd",
                SymbolKind::Function => "fn",
                SymbolKind::Local => "lc",
                SymbolKind::Module => "md",
                SymbolKind::Struct => "st",
                SymbolKind::TypeAlias => "ta",
            },
            CompletionItemKind::Attribute => "at",
            CompletionItemKind::Binding => "bn",
            CompletionItemKind::BuiltinType => "bt",
            CompletionItemKind::Keyword => "kw",
            CompletionItemKind::Method => "me",
            CompletionItemKind::Snippet => "sn",
            CompletionItemKind::UnresolvedReference => "??",
        }
    }
}

impl From<SymbolKind> for CompletionItemKind {
    fn from(kind: SymbolKind) -> Self {
        CompletionItemKind::SymbolKind(kind)
    }
}

impl CompletionItem {
    /// Constructs a [`Builder`] to build a `CompletionItem` with
    pub fn builder(kind: CompletionKind, label: impl Into<String>) -> Builder {
        Builder {
            label: label.into(),
            kind: None,
            completion_kind: kind,
            detail: None,
        }
    }
}

/// A builder for a `CompletionItem`. Constructed by calling
/// [`CompletionItem::builder`].
pub struct Builder {
    label: String,
    completion_kind: CompletionKind,
    kind: Option<CompletionItemKind>,
    detail: Option<String>,
}

impl Builder {
    /// Completes building the `CompletionItem` and returns it
    pub fn finish(self) -> CompletionItem {
        CompletionItem {
            completion_kind: self.completion_kind,
            label: self.label,
            kind: self.kind,
            detail: self.detail,
        }
    }

    /// Sets the type of the completion
    pub fn kind(mut self, kind: impl Into<CompletionItemKind>) -> Builder {
        self.kind = Some(kind.into());
        self
    }

    /// Set the details of the completion item
    pub fn detail(mut self, detail: impl Into<String>) -> Builder {
        self.detail = Some(detail.into());
        self
    }
}

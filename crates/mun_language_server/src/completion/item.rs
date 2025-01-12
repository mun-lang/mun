use crate::SymbolKind;

/// A `CompletionItem` describes a single completion variant in an editor.
#[derive(Clone, Debug)]
pub struct CompletionItem {
    /// Label in the completion pop up which identifies completion.
    pub label: String,

    /// The type of completion
    pub kind: CompletionItemKind,

    /// Additional info to show in the UI pop up.
    pub detail: Option<String>,
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
                SymbolKind::SelfParam => "sp",
                SymbolKind::SelfType => "sy",
                SymbolKind::Struct => "st",
                SymbolKind::TypeAlias => "ta",
                SymbolKind::Impl => "im",
                SymbolKind::Method => "mt",
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
    pub fn builder(kind: impl Into<CompletionItemKind>, label: impl Into<String>) -> Builder {
        Builder {
            label: label.into(),
            kind: kind.into(),
            detail: None,
        }
    }
}

/// A builder for a `CompletionItem`. Constructed by calling
/// [`CompletionItem::builder`].
pub struct Builder {
    label: String,
    kind: CompletionItemKind,
    detail: Option<String>,
}

impl Builder {
    /// Completes building the `CompletionItem` and returns it
    pub fn finish(self) -> CompletionItem {
        CompletionItem {
            label: self.label,
            kind: self.kind,
            detail: self.detail,
        }
    }

    /// Set the details of the completion item
    pub fn detail(mut self, detail: impl Into<String>) -> Builder {
        self.detail = Some(detail.into());
        self
    }
}

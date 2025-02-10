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

    /// The relevance of this completion item. This is used to score different
    /// completions.
    pub relevance: CompletionRelevance,
}

/// Defines the relevance of a completion item this is used to score different
/// completions amongst each-other.
#[derive(Clone, Debug, Default)]
pub struct CompletionRelevance {
    /// True for local variables.
    pub is_local: bool,
}

impl CompletionRelevance {
    /// Returns a relative score for the completion item. This is used to sort
    /// the completions.
    pub fn score(&self) -> u32 {
        let mut score = !0 / 2;
        let CompletionRelevance { is_local } = self;

        // Slightly prefer locals
        if *is_local {
            score += 1;
        }

        score
    }

    /// Indicates whether this item likely especially relevant to the user.
    pub fn is_relevant(&self) -> bool {
        self.score() > 0
    }
}

/// Type of completion used to provide hints to the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
            relevance: CompletionRelevance::default(),
        }
    }
}

/// A builder for a `CompletionItem`. Constructed by calling
/// [`CompletionItem::builder`].
pub struct Builder {
    label: String,
    kind: CompletionItemKind,
    detail: Option<String>,
    relevance: CompletionRelevance,
}

impl Builder {
    /// Completes building the `CompletionItem` and returns it
    pub fn finish(self) -> CompletionItem {
        CompletionItem {
            label: self.label,
            kind: self.kind,
            detail: self.detail,
            relevance: self.relevance,
        }
    }

    /// Set the details of the completion item
    pub fn with_detail(mut self, detail: impl Into<String>) -> Builder {
        self.detail = Some(detail.into());
        self
    }

    /// Set the details of the completion item
    pub fn set_detail(&mut self, detail: impl Into<String>) {
        self.detail = Some(detail.into());
    }

    /// Sets the relevance of this item
    pub fn set_relevance(&mut self, relevance: CompletionRelevance) {
        self.relevance = relevance;
    }
}

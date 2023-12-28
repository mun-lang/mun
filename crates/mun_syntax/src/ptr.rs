use crate::{AstNode, SyntaxKind, SyntaxNode, TextRange};
use std::hash::{Hash, Hasher};
use std::iter::successors;
use std::marker::PhantomData;

/// A pointer to a syntax node inside a file. It can be used to remember a
/// specific node across reparses of the same file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SyntaxNodePtr {
    pub(crate) range: TextRange,
    kind: SyntaxKind,
}

impl SyntaxNodePtr {
    pub fn new(node: &SyntaxNode) -> SyntaxNodePtr {
        SyntaxNodePtr {
            range: node.text_range(),
            kind: node.kind(),
        }
    }

    pub fn to_node(&self, root: &SyntaxNode) -> SyntaxNode {
        assert!(root.parent().is_none());
        successors(Some(root.clone()), |node| {
            node.children()
                .find(|it| it.text_range().contains_range(self.range))
        })
        .find(|it| it.text_range() == self.range && it.kind() == self.kind)
        .unwrap_or_else(|| panic!("can't resolve local ptr to SyntaxNode: {self:?}"))
    }

    pub fn range(&self) -> TextRange {
        self.range
    }

    pub fn kind(&self) -> SyntaxKind {
        self.kind
    }

    /// Casts this instance to a pointer of another type. Returns None if the cast is not valid.
    pub fn try_cast<N: AstNode>(self) -> Option<AstPtr<N>> {
        if !N::can_cast(self.kind) {
            return None;
        }
        Some(AstPtr {
            raw: self,
            _ty: PhantomData,
        })
    }
}

/// Like `SyntaxNodePtr`, but remembers the type of node
#[derive(Debug)]
pub struct AstPtr<N: AstNode> {
    raw: SyntaxNodePtr,
    _ty: PhantomData<fn() -> N>,
}

impl<N: AstNode> Clone for AstPtr<N> {
    fn clone(&self) -> AstPtr<N> {
        AstPtr {
            raw: self.raw.clone(),
            _ty: PhantomData,
        }
    }
}

impl<N: AstNode> Eq for AstPtr<N> {}

impl<N: AstNode> PartialEq for AstPtr<N> {
    fn eq(&self, other: &AstPtr<N>) -> bool {
        self.raw == other.raw
    }
}

impl<N: AstNode> Hash for AstPtr<N> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl<N: AstNode> AstPtr<N> {
    pub fn new(node: &N) -> AstPtr<N> {
        AstPtr {
            raw: SyntaxNodePtr::new(node.syntax()),
            _ty: PhantomData,
        }
    }

    pub fn to_node(&self, root: &SyntaxNode) -> N {
        let syntax_node = self.raw.to_node(root);
        N::cast(syntax_node).unwrap()
    }

    pub fn syntax_node_ptr(&self) -> SyntaxNodePtr {
        self.raw.clone()
    }

    pub fn cast<U: AstNode>(self) -> Option<AstPtr<U>> {
        if !U::can_cast(self.raw.kind) {
            return None;
        }
        Some(AstPtr {
            raw: self.raw,
            _ty: PhantomData,
        })
    }
}

impl<N: AstNode> From<AstPtr<N>> for SyntaxNodePtr {
    fn from(ptr: AstPtr<N>) -> SyntaxNodePtr {
        ptr.raw
    }
}

use std::{
    hash::{Hash, Hasher},
    marker::PhantomData,
    sync::Arc,
};

use mun_syntax::{ast, AstNode, AstPtr, SyntaxNode, SyntaxNodePtr, WalkEvent};

use crate::{
    arena::{Arena, Idx},
    db::AstDatabase,
    in_file::InFile,
    FileId,
};

type ErasedFileAstId = Idx<SyntaxNodePtr>;

/// `AstId` points to an AST node in any file.
///
/// It is stable across reparses, and can be used as salsa key/value.
pub(crate) type AstId<N> = InFile<FileAstId<N>>;

impl<N: AstIdNode> AstId<N> {
    pub fn to_node(self, db: &dyn AstDatabase) -> N {
        let root = db.parse(self.file_id);
        db.ast_id_map(self.file_id)
            .get(self.value)
            .to_node(&root.syntax_node())
    }
}

#[derive(Clone, Debug)]
pub struct FileAstId<N: AstIdNode> {
    raw: ErasedFileAstId,
    _ty: PhantomData<fn() -> N>,
}

impl<N: AstIdNode> Copy for FileAstId<N> {}

impl<N: AstIdNode> PartialEq for FileAstId<N> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}
impl<N: AstIdNode> Eq for FileAstId<N> {}
impl<N: AstIdNode> Hash for FileAstId<N> {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.raw.hash(hasher);
    }
}

/// A trait that is implemented for all nodes that can be represented as a
/// `FileAstId`.
pub trait AstIdNode: AstNode {}

macro_rules! register_ast_id_node {
    (impl AstIdNode for $($ident:ident),+ ) => {
        $(
            impl AstIdNode for ast::$ident {}
        )+
        fn should_alloc_id(kind: mun_syntax::SyntaxKind) -> bool {
            $(
                ast::$ident::can_cast(kind)
            )||+
        }
    };
}

register_ast_id_node! {
    impl AstIdNode for
    ModuleItem,
        Use,
        FunctionDef,
        StructDef,
        Impl,
        TypeAliasDef,
    Param
}

/// Maps items' `SyntaxNode`s to `ErasedFileAstId`s and back.
#[derive(Debug, PartialEq, Eq, Default)]
pub struct AstIdMap {
    arena: Arena<SyntaxNodePtr>,
}

impl AstIdMap {
    pub(crate) fn ast_id_map_query(db: &dyn AstDatabase, file_id: FileId) -> Arc<AstIdMap> {
        let map = AstIdMap::from_source(db.parse(file_id).tree().syntax());
        Arc::new(map)
    }

    pub(crate) fn ast_id<N: AstIdNode>(&self, item: &N) -> FileAstId<N> {
        let ptr = SyntaxNodePtr::new(item.syntax());
        let raw = match self.arena.iter().find(|(_id, i)| **i == ptr) {
            Some((it, _)) => it,
            None => panic!(
                "Can't find {:?} in AstIdMap:\n{:?}",
                item.syntax(),
                self.arena.iter().map(|(_id, i)| i).collect::<Vec<_>>(),
            ),
        };

        FileAstId {
            raw,
            _ty: PhantomData,
        }
    }

    /// Constructs a new `AstIdMap` from a root [`SyntaxNode`].
    /// `node` must be the root of a syntax tree.
    fn from_source(node: &SyntaxNode) -> AstIdMap {
        assert!(node.parent().is_none());
        let mut res = AstIdMap::default();

        // Make sure the root node is allocated
        if !should_alloc_id(node.kind()) {
            res.alloc(node);
        }

        // By walking the tree in breadth-first order we make sure that parents
        // get lower ids then children. That is, adding a new child does not
        // change parent's id. This means that, say, adding a new function to a
        // trait does not change ids of top-level items, which helps caching.
        bdfs(node, |it| {
            if should_alloc_id(it.kind()) {
                res.alloc(&it);
                TreeOrder::BreadthFirst
            } else {
                TreeOrder::DepthFirst
            }
        });

        res
    }

    /// Returns the `AstPtr` of the given id.
    pub(crate) fn get<N: AstIdNode>(&self, id: FileAstId<N>) -> AstPtr<N> {
        self.arena[id.raw].clone().try_cast::<N>().unwrap()
    }

    /// Constructs a new `ErasedFileAstId` from a `SyntaxNode`
    fn alloc(&mut self, item: &SyntaxNode) -> ErasedFileAstId {
        self.arena.alloc(SyntaxNodePtr::new(item))
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum TreeOrder {
    BreadthFirst,
    DepthFirst,
}

/// Walks the subtree in bdfs order, calling `f` for each node.
///
/// ### What is bdfs order?
///
/// It is a mix of breadth-first and depth first orders. Nodes for which `f`
/// returns [`TreeOrder::BreadthFirst`] are visited breadth-first, all the other
/// nodes are explored [`TreeOrder::DepthFirst`].
///
/// In other words, the size of the bfs queue is bound by the number of "true"
/// nodes.
fn bdfs(node: &SyntaxNode, mut f: impl FnMut(SyntaxNode) -> TreeOrder) {
    let mut curr_layer = vec![node.clone()];
    let mut next_layer = vec![];
    while !curr_layer.is_empty() {
        curr_layer.drain(..).for_each(|node| {
            let mut preorder = node.preorder();
            while let Some(event) = preorder.next() {
                match event {
                    WalkEvent::Enter(node) => {
                        if f(node.clone()) == TreeOrder::BreadthFirst {
                            next_layer.extend(node.children());
                            preorder.skip_subtree();
                        }
                    }
                    WalkEvent::Leave(_) => {}
                }
            }
        });
        std::mem::swap(&mut curr_layer, &mut next_layer);
    }
}

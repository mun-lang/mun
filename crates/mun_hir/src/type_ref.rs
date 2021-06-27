//! HIR for references to types. These paths are not yet resolved. They can be directly created
//! from an `ast::TypeRef`, without further queries.

use crate::{
    arena::{map::ArenaMap, Arena, Idx},
    Path,
};
use mun_syntax::{ast, AstPtr};
use rustc_hash::FxHashMap;
use std::ops::Index;

/// The ID of a `TypeRef` in a `TypeRefMap`
pub type LocalTypeRefId = Idx<TypeRef>;

/// Compare ty::Ty
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum TypeRef {
    Path(Path),
    Array(LocalTypeRefId),
    Never,
    Tuple(Vec<LocalTypeRefId>),
    Error,
}

#[derive(Default, Debug, Eq, PartialEq, Clone)]
pub struct TypeRefSourceMap {
    type_ref_map: FxHashMap<AstPtr<ast::TypeRef>, LocalTypeRefId>,
    type_ref_map_back: ArenaMap<LocalTypeRefId, AstPtr<ast::TypeRef>>,
}

impl TypeRefSourceMap {
    /// Returns the syntax node of the specified `LocalTypeRefId` or `None` if it doesnt exist in
    /// this instance.
    pub(crate) fn type_ref_syntax(&self, expr: LocalTypeRefId) -> Option<AstPtr<ast::TypeRef>> {
        self.type_ref_map_back.get(expr).cloned()
    }

    /// Returns the `LocalTypeRefId` references at the given location or `None` if no such Id
    /// exists.
    pub(crate) fn syntax_type_ref(&self, ptr: AstPtr<ast::TypeRef>) -> Option<LocalTypeRefId> {
        self.type_ref_map.get(&ptr).cloned()
    }
}

/// Holds all type references from a specific region in the source code (depending on the use of
/// this struct). This struct is often used in conjunction with a `TypeRefSourceMap` which maps
/// `LocalTypeRefId`s to location in the syntax tree and back.
#[derive(Default, Debug, Eq, PartialEq, Clone)]
pub struct TypeRefMap {
    type_refs: Arena<TypeRef>,
}

impl TypeRefMap {
    pub(crate) fn builder() -> TypeRefMapBuilder {
        TypeRefMapBuilder {
            map: Default::default(),
            source_map: Default::default(),
        }
    }

    /// Returns an iterator over all types in this instance
    pub fn iter(&self) -> impl Iterator<Item = (LocalTypeRefId, &TypeRef)> {
        self.type_refs.iter()
    }
}

impl Index<LocalTypeRefId> for TypeRefMap {
    type Output = TypeRef;

    fn index(&self, pat: LocalTypeRefId) -> &Self::Output {
        &self.type_refs[pat]
    }
}

/// A builder object to lower type references from syntax to a more abstract representation.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct TypeRefMapBuilder {
    map: TypeRefMap,
    source_map: TypeRefSourceMap,
}

impl TypeRefMapBuilder {
    /// Allocates a new `LocalTypeRefId` for the specified `TypeRef`. The passed `ptr` marks where
    /// the `TypeRef` is located in the AST.
    fn alloc_type_ref(&mut self, type_ref: TypeRef, ptr: AstPtr<ast::TypeRef>) -> LocalTypeRefId {
        let id = self.map.type_refs.alloc(type_ref);
        self.source_map.type_ref_map.insert(ptr.clone(), id);
        self.source_map.type_ref_map_back.insert(id, ptr);
        id
    }

    /// Lowers the given optional AST type references and returns the Id of the resulting `TypeRef`.
    /// If the node is None an error is created indicating a missing `TypeRef` in the AST.
    pub fn alloc_from_node_opt(&mut self, node: Option<&ast::TypeRef>) -> LocalTypeRefId {
        if let Some(node) = node {
            self.alloc_from_node(node)
        } else {
            self.error()
        }
    }

    /// Lowers the given AST type references and returns the Id of the resulting `TypeRef`.
    pub fn alloc_from_node(&mut self, node: &ast::TypeRef) -> LocalTypeRefId {
        use mun_syntax::ast::TypeRefKind::*;
        let ptr = AstPtr::new(node);
        let type_ref = match node.kind() {
            PathType(path) => path
                .path()
                .and_then(Path::from_ast)
                .map(TypeRef::Path)
                .unwrap_or(TypeRef::Error),
            NeverType(_) => TypeRef::Never,
            ArrayType(inner) => TypeRef::Array(self.alloc_from_node_opt(inner.type_ref().as_ref())),
        };
        self.alloc_type_ref(type_ref, ptr)
    }

    /// Constructs a new `TypeRef` for the empty tuple type. Returns the Id of the newly create
    /// `TypeRef`.
    pub fn unit(&mut self) -> LocalTypeRefId {
        self.map.type_refs.alloc(TypeRef::Tuple(vec![]))
    }

    /// Constructs a new error `TypeRef` which marks an error in the AST.
    pub fn error(&mut self) -> LocalTypeRefId {
        self.map.type_refs.alloc(TypeRef::Error)
    }

    /// Finish building type references, returning the `TypeRefMap` which contains all the
    /// `TypeRef`s and a `TypeRefSourceMap` which converts LocalTypeRefIds back to source location.
    pub fn finish(self) -> (TypeRefMap, TypeRefSourceMap) {
        (self.map, self.source_map)
    }
}

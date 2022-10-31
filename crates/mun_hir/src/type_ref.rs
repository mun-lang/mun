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
    Never,
    Empty,
    Error,
}

impl TypeRef {
    /// Converts an `ast::TypeRef` to a `mun_hir::TypeRef`.
    pub fn from_ast(node: ast::TypeRef) -> Self {
        match node.kind() {
            ast::TypeRefKind::NeverType(..) => TypeRef::Never,
            ast::TypeRefKind::PathType(inner) => {
                // FIXME: Use `Path::from_src`
                inner
                    .path()
                    .and_then(Path::from_ast)
                    .map(TypeRef::Path)
                    .unwrap_or(TypeRef::Error)
            }
        }
    }

    pub fn from_ast_opt(node: Option<ast::TypeRef>) -> Self {
        if let Some(node) = node {
            TypeRef::from_ast(node)
        } else {
            TypeRef::Error
        }
    }
}

#[derive(Default, Debug, Eq, PartialEq)]
pub struct TypeRefSourceMap {
    type_ref_map: FxHashMap<AstPtr<ast::TypeRef>, LocalTypeRefId>,
    type_ref_map_back: ArenaMap<LocalTypeRefId, AstPtr<ast::TypeRef>>,
}

impl TypeRefSourceMap {
    pub(crate) fn type_ref_syntax(&self, expr: LocalTypeRefId) -> Option<AstPtr<ast::TypeRef>> {
        self.type_ref_map_back.get(expr).cloned()
    }

    pub(crate) fn syntax_type_ref(&self, ptr: AstPtr<ast::TypeRef>) -> Option<LocalTypeRefId> {
        self.type_ref_map.get(&ptr).cloned()
    }
}

#[derive(Default, Debug, Eq, PartialEq)]
pub struct TypeRefMap {
    type_refs: Arena<TypeRef>,
}

impl TypeRefMap {
    /// Iterate over the elements in the map
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

#[derive(Default, Debug, Eq, PartialEq)]
pub(crate) struct TypeRefBuilder {
    map: TypeRefMap,
    source_map: TypeRefSourceMap,
}

impl TypeRefBuilder {
    fn alloc_type_ref(&mut self, type_ref: TypeRef, ptr: AstPtr<ast::TypeRef>) -> LocalTypeRefId {
        let id = self.map.type_refs.alloc(type_ref);
        self.source_map.type_ref_map.insert(ptr, id);
        self.source_map.type_ref_map_back.insert(id, ptr);
        id
    }

    pub fn alloc_from_node_opt(&mut self, node: Option<&ast::TypeRef>) -> LocalTypeRefId {
        if let Some(node) = node {
            self.alloc_from_node(node)
        } else {
            self.error()
        }
    }

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
        };
        self.alloc_type_ref(type_ref, ptr)
    }

    pub fn unit(&mut self) -> LocalTypeRefId {
        self.map.type_refs.alloc(TypeRef::Empty)
    }

    pub fn error(&mut self) -> LocalTypeRefId {
        self.map.type_refs.alloc(TypeRef::Error)
    }

    pub fn finish(self) -> (TypeRefMap, TypeRefSourceMap) {
        (self.map, self.source_map)
    }
}

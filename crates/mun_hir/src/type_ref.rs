use crate::arena::map::ArenaMap;
use crate::arena::{Arena, RawId};
///! HIR for references to types. These paths are not yet resolved. They can be directly created
/// from an `ast::TypeRef`, without further queries.
use crate::Path;
use mun_syntax::ast;
use mun_syntax::AstPtr;
use rustc_hash::FxHashMap;
use std::ops::Index;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypeRefId(RawId);
impl_arena_id!(TypeRefId);

/// Compare ty::Ty
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum TypeRef {
    Path(Path),
    Empty,
    Error,
}

#[derive(Default, Debug, Eq, PartialEq)]
pub struct TypeRefSourceMap {
    type_ref_map: FxHashMap<AstPtr<ast::TypeRef>, TypeRefId>,
    type_ref_map_back: ArenaMap<TypeRefId, AstPtr<ast::TypeRef>>,
}

impl TypeRefSourceMap {
    pub(crate) fn type_ref_syntax(&self, expr: TypeRefId) -> Option<AstPtr<ast::TypeRef>> {
        self.type_ref_map_back.get(expr).cloned()
    }

    pub(crate) fn syntax_type_ref(&self, ptr: AstPtr<ast::TypeRef>) -> Option<TypeRefId> {
        self.type_ref_map.get(&ptr).cloned()
    }
}

#[derive(Default, Debug, Eq, PartialEq)]
pub struct TypeRefMap {
    type_refs: Arena<TypeRefId, TypeRef>,
}

impl Index<TypeRefId> for TypeRefMap {
    type Output = TypeRef;

    fn index(&self, pat: TypeRefId) -> &Self::Output {
        &self.type_refs[pat]
    }
}

#[derive(Default, Debug, Eq, PartialEq)]
pub(crate) struct TypeRefBuilder {
    map: TypeRefMap,
    source_map: TypeRefSourceMap,
}

impl TypeRefBuilder {
    fn alloc_type_ref(&mut self, type_ref: TypeRef, ptr: AstPtr<ast::TypeRef>) -> TypeRefId {
        let id = self.map.type_refs.alloc(type_ref);
        self.source_map.type_ref_map.insert(ptr, id);
        self.source_map.type_ref_map_back.insert(id, ptr);
        id
    }

    pub fn from_node_opt(&mut self, node: Option<&ast::TypeRef>) -> TypeRefId {
        if let Some(node) = node {
            self.from_node(node)
        } else {
            self.error()
        }
    }

    pub fn from_node(&mut self, node: &ast::TypeRef) -> TypeRefId {
        use mun_syntax::ast::TypeRefKind::*;
        let ptr = AstPtr::new(node);
        let type_ref = match node.kind() {
            PathType(path) => path
                .path()
                .and_then(Path::from_ast)
                .map(TypeRef::Path)
                .unwrap_or(TypeRef::Error),
        };
        self.alloc_type_ref(type_ref, ptr)
    }

    pub fn unit(&mut self) -> TypeRefId {
        self.map.type_refs.alloc(TypeRef::Empty)
    }

    pub fn error(&mut self) -> TypeRefId {
        self.map.type_refs.alloc(TypeRef::Error)
    }

    pub fn finish(self) -> (TypeRefMap, TypeRefSourceMap) {
        (self.map, self.source_map)
    }
}

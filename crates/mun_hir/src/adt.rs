use std::sync::Arc;

use crate::{
    arena::{Arena, RawId},
    ids::{AstItemDef, StructId},
    type_ref::TypeRef,
    AsName, DefDatabase, Name,
};
use mun_syntax::ast::{self, NameOwner, TypeAscriptionOwner};

/// A single field of an enum variant or struct
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldData {
    pub name: Name,
    pub type_ref: TypeRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalStructFieldId(RawId);
impl_arena_id!(LocalStructFieldId);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructData {
    pub name: Name,
    pub fields: Option<Arc<Arena<LocalStructFieldId, FieldData>>>,
}

impl StructData {
    pub(crate) fn struct_data_query(db: &impl DefDatabase, id: StructId) -> Arc<StructData> {
        let src = id.source(db);
        let name = src
            .ast
            .name()
            .map(|n| n.as_name())
            .unwrap_or_else(Name::missing);

        let fields = if let ast::StructKind::Record(r) = src.ast.kind() {
            let fields = r
                .fields()
                .map(|fd| FieldData {
                    name: fd.name().map(|n| n.as_name()).unwrap_or_else(Name::missing),
                    type_ref: TypeRef::from_ast_opt(fd.ascribed_type()),
                })
                .collect();
            Some(Arc::new(fields))
        } else {
            None
        };
        Arc::new(StructData { name, fields })
    }
}

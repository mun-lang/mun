use std::sync::Arc;

use crate::type_ref::{TypeRefBuilder, TypeRefId, TypeRefMap, TypeRefSourceMap};
use crate::{
    arena::{Arena, RawId},
    ids::{AstItemDef, StructId},
    AsName, DefDatabase, Name,
};
use mun_syntax::ast::{self, NameOwner, TypeAscriptionOwner};

/// A single field of a record
/// ```mun
/// struct Foo {
///     a: int, // <- this
/// }
/// ```
/// or
/// ```mun
/// struct Foo(
///     int, // <- this
/// )
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructFieldData {
    pub name: Name,
    pub type_ref: TypeRefId,
}

/// An identifier for a struct's or tuple's field
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StructFieldId(RawId);
impl_arena_id!(StructFieldId);

/// A struct's fields' data (record, tuple, or unit struct)
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StructKind {
    Record,
    Tuple,
    Unit,
}

#[derive(Debug, PartialEq, Eq)]
pub struct StructData {
    pub name: Name,
    pub fields: Arena<StructFieldId, StructFieldData>,
    pub kind: StructKind,
    type_ref_map: TypeRefMap,
    type_ref_source_map: TypeRefSourceMap,
}

impl StructData {
    pub(crate) fn struct_data_query(db: &impl DefDatabase, id: StructId) -> Arc<StructData> {
        let src = id.source(db);
        let name = src
            .ast
            .name()
            .map(|n| n.as_name())
            .unwrap_or_else(Name::missing);

        let mut type_ref_builder = TypeRefBuilder::default();
        let (fields, kind) = match src.ast.kind() {
            ast::StructKind::Record(r) => {
                let fields = r
                    .fields()
                    .map(|fd| StructFieldData {
                        name: fd.name().map(|n| n.as_name()).unwrap_or_else(Name::missing),
                        type_ref: type_ref_builder.alloc_from_node_opt(fd.ascribed_type().as_ref()),
                    })
                    .collect();
                (fields, StructKind::Record)
            }
            ast::StructKind::Tuple(t) => {
                let fields = t
                    .fields()
                    .enumerate()
                    .map(|(index, fd)| StructFieldData {
                        name: Name::new_tuple_field(index),
                        type_ref: type_ref_builder.alloc_from_node_opt(fd.type_ref().as_ref()),
                    })
                    .collect();
                (fields, StructKind::Tuple)
            }
            ast::StructKind::Unit => (Arena::default(), StructKind::Unit),
        };

        let (type_ref_map, type_ref_source_map) = type_ref_builder.finish();
        Arc::new(StructData {
            name,
            fields,
            kind,
            type_ref_map,
            type_ref_source_map,
        })
    }

    pub fn type_ref_source_map(&self) -> &TypeRefSourceMap {
        &self.type_ref_source_map
    }

    pub fn type_ref_map(&self) -> &TypeRefMap {
        &self.type_ref_map
    }
}

use std::sync::Arc;

use la_arena::Arena;
use mun_syntax::ast::{self, NameOwner, TypeAscriptionOwner as _, VisibilityOwner as _};

use super::r#struct::FieldData;
use crate::{
    has_module::HasModule as _,
    ids::{EnumId, EnumVariantId, EnumVariantLoc, Intern as _, Lookup},
    item_tree::ItemTreeId,
    name::AsName,
    type_ref::{TypeRefMap, TypeRefSourceMap},
    visibility::RawVisibility,
    DefDatabase, HirDatabase, Module, Name,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Enum {
    id: EnumId,
}

impl Enum {
    pub fn module(self, db: &dyn HirDatabase) -> Module {
        self.id.module(db.upcast()).into()
    }

    pub fn name(self, db: &dyn HirDatabase) -> Name {
        db.enum_data(self.id).name.clone()
    }
}

impl From<EnumId> for Enum {
    fn from(id: EnumId) -> Self {
        Enum { id }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct EnumData {
    pub name: Name,
    pub variants: Box<[EnumVariantId]>,
    pub visibility: RawVisibility,
}

impl EnumData {
    pub(crate) fn enum_data_query(db: &dyn DefDatabase, id: EnumId) -> Arc<EnumData> {
        let loc = id.lookup(db);
        let item_tree = db.item_tree(loc.id.file_id);

        let item = &item_tree[loc.id.value];
        let src = item_tree.source(db, loc.id.value);

        // TODO: Should I keep this here or use the one in `collector.rs:556`?
        let mut index = 0;
        let variants = item
            .variants
            .clone()
            .map(|variant| {
                let loc = EnumVariantLoc {
                    id: ItemTreeId::new(loc.id.file_id, variant.into()),
                    parent: id,
                    index,
                }
                .intern(db);

                index += 1;
                loc
            })
            .collect();

        Arc::new(EnumData {
            name: item.name.clone(),
            variants,
            visibility: item_tree[item.visibility],
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct EnumVariantData {
    pub name: Name,
    pub data: VariantData,
    // TODO: Can we avoid storing these on the enum variant?
    // TODO: RA uses `Interned` for this. Can we do the same?
    type_ref_map: TypeRefMap,
    type_ref_source_map: TypeRefSourceMap,
}

impl EnumVariantData {
    pub(crate) fn query(db: &dyn DefDatabase, id: EnumVariantId) -> Arc<EnumVariantData> {
        let loc = id.lookup(db);
        let item_tree = db.item_tree(loc.id.file_id);

        let variant = &item_tree[loc.id.value];
        let src = item_tree.source(db, loc.id.value);

        let mut type_ref_builder = TypeRefMap::builder();
        let data = match src.kind() {
            ast::StructKind::Record(record) => {
                let fields = record
                    .fields()
                    .map(|field| FieldData {
                        name: field
                            .name()
                            .map_or_else(Name::missing, |name| name.as_name()),
                        type_ref: type_ref_builder
                            .alloc_from_node_opt(field.ascribed_type().as_ref()),
                        visibility: RawVisibility::from_ast(field.visibility()),
                    })
                    .collect();

                VariantData::Record(fields)
            }
            ast::StructKind::Tuple(tuple) => {
                let fields = tuple
                    .fields()
                    .enumerate()
                    .map(|(index, field)| FieldData {
                        name: Name::new_tuple_field(index),
                        type_ref: type_ref_builder.alloc_from_node_opt(field.type_ref().as_ref()),
                        visibility: RawVisibility::from_ast(field.visibility()),
                    })
                    .collect();

                VariantData::Tuple(fields)
            }
            ast::StructKind::Unit => VariantData::Unit,
        };

        let (type_ref_map, type_ref_source_map) = type_ref_builder.finish();
        Arc::new(EnumVariantData {
            name: variant.name,
            data,
            type_ref_map,
            type_ref_source_map,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VariantData {
    Record(Arena<FieldData>),
    Tuple(Arena<FieldData>),
    Unit,
}

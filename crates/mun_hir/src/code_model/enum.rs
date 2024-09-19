use std::sync::Arc;

use la_arena::Arena;
use mun_syntax::ast::{self, NameOwner, TypeAscriptionOwner as _, VisibilityOwner as _};

use super::{field::FieldsData, r#struct::FieldData};
use crate::{
    code_model::field::Field,
    has_module::HasModule as _,
    ids::{EnumId, EnumVariantId, EnumVariantLoc, Intern as _, Lookup},
    item_tree::ItemTreeId,
    name::AsName,
    name_resolution::Namespace,
    type_ref::{TypeRefMap, TypeRefSourceMap},
    visibility::RawVisibility,
    DefDatabase, HirDatabase, Module, Name, Ty,
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

    pub fn variants(self, db: &dyn HirDatabase) -> Box<[EnumVariant]> {
        db.enum_data(self.id)
            .variants
            .iter()
            .map(|&id| id.into())
            .collect()
    }

    pub fn ty(self, db: &dyn HirDatabase) -> Ty {
        db.type_for_def(self.into(), Namespace::Types)
    }
}

impl From<EnumId> for Enum {
    fn from(id: EnumId) -> Self {
        Enum { id }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EnumVariant {
    id: EnumVariantId,
}

impl EnumVariant {
    pub fn fields(self, db: &dyn HirDatabase) -> Box<[Field]> {
        self.data(db)
            .fields()
            .iter()
            .map(|(id, _)| Field {
                parent: self.into(),
                id,
            })
            .collect()
    }

    pub(crate) fn data(self, db: &dyn HirDatabase) -> Arc<EnumVariantData> {
        db.enum_variant_data(self.id)
    }
}

impl From<EnumVariant> for EnumVariantId {
    fn from(value: EnumVariant) -> Self {
        value.id
    }
}

impl From<EnumVariantId> for EnumVariant {
    fn from(id: EnumVariantId) -> Self {
        EnumVariant { id }
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
            visibility: item_tree[item.visibility].clone(),
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct EnumVariantData {
    pub name: Name,
    pub fields_data: Arc<FieldsData>,
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
        let fields = match src.kind() {
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

                FieldsData::Record(fields)
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

                FieldsData::Tuple(fields)
            }
            ast::StructKind::Unit => FieldsData::Unit,
        };

        let (type_ref_map, type_ref_source_map) = type_ref_builder.finish();
        Arc::new(EnumVariantData {
            name: variant.name.clone(),
            fields_data: Arc::new(fields),
            type_ref_map,
            type_ref_source_map,
        })
    }

    pub fn fields(&self) -> &Arena<FieldData> {
        self.fields_data.fields()
    }
}

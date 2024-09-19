use std::sync::Arc;

use la_arena::{Arena, Idx};

use super::{r#enum::EnumVariant, r#struct::FieldData, StructKind};
use crate::{ids::FieldOwnerId, ty::lower::LowerTyMap, HirDatabase, Name, Struct, Ty};

/// An identifier for a struct's or tuple's field
pub type LocalFieldId = Idx<FieldData>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldsData {
    Record(Arena<FieldData>),
    Tuple(Arena<FieldData>),
    Unit,
}

impl FieldsData {
    pub fn fields(&self) -> &Arena<FieldData> {
        const EMPTY: &Arena<FieldData> = &Arena::new();

        match self {
            FieldsData::Record(fields) | FieldsData::Tuple(fields) => fields,
            FieldsData::Unit => EMPTY,
        }
    }

    pub fn kind(&self) -> StructKind {
        match self {
            FieldsData::Record(_) => StructKind::Record,
            FieldsData::Tuple(_) => StructKind::Tuple,
            FieldsData::Unit => StructKind::Unit,
        }
    }
}

/// A field of a [`Struct`], [`Variant`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Field {
    pub(crate) parent: FieldOwner,
    pub(crate) id: LocalFieldId,
}

impl Field {
    /// Returns the type of the field
    pub fn ty(self, db: &dyn HirDatabase) -> Ty {
        let fields = self.parent.fields(db);
        let type_ref_id = fields[self.id].type_ref;
        let fields = self.parent.lower(db);
        lower[type_ref_id].clone()
    }

    /// Returns the name of the field
    pub fn name(self, db: &dyn HirDatabase) -> Name {
        self.parent.fields(db)[self.id].name.clone()
    }

    /// Returns the index of this field in the parent
    pub fn index(self, _db: &dyn HirDatabase) -> u32 {
        self.id.into_raw().into()
    }

    /// Returns the ID of the field with relation to the parent struct
    pub(crate) fn id(self) -> LocalFieldId {
        self.id
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldOwner {
    EnumVariant(EnumVariant),
    Struct(Struct),
}

impl_froms!(EnumVariant, Struct for FieldOwner);

impl FieldOwner {
    pub fn fields_data(self, db: &dyn HirDatabase) -> Arc<FieldsData> {
        match self {
            FieldOwner::EnumVariant(v) => v.data(db).fields_data.clone(),
            FieldOwner::Struct(s) => s.data(db.upcast()).fields_data.clone(),
        }
    }

    pub fn fields(&self, db: &dyn HirDatabase) -> &Arena<FieldData> {
        self.fields_data(db).fields()
    }
}

impl From<FieldOwner> for FieldOwnerId {
    fn from(value: FieldOwner) -> Self {
        match value {
            FieldOwner::EnumVariant(e) => Self::EnumVariantId(e.into()),
            FieldOwner::Struct(s) => Self::StructId(s.id),
        }
    }
}

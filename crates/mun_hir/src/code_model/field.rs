use la_arena::{Arena, Idx};

use super::{r#enum::Variant, r#struct::FieldData};
use crate::{HirDatabase, Name, Struct, Ty};

/// An identifier for a struct's or tuple's field
pub type LocalFieldId = Idx<FieldData>;

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
        let lower = self.parent.lower(db);
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
    Struct(Struct),
    Variant(Variant),
}

impl_froms!(Struct, Variant for FieldOwner);

impl FieldOwner {
    pub fn fields(self, db: &dyn HirDatabase) -> &Arena<FieldData> {
        match self {
            FieldOwner::Struct(s) => &s.data(db.upcast()).fields,
            FieldOwner::Variant(v) => v.data(db).fields(),
        }
    }
}

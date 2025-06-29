use c_codegen::r#type::{member::Member, structure::Struct};
use mun_hir::HirDatabase;

use crate::{
    identifier::{field_name_to_identifier, full_name_to_identifier},
    ty,
};

pub fn declaration(db: &dyn HirDatabase, ty: mun_hir::Struct) -> Struct {
    Struct::Tag {
        name: full_name_to_identifier(&ty.full_name(db)),
    }
}

pub fn definition(db: &dyn HirDatabase, ty: mun_hir::Struct) -> Struct {
    let members = ty
        .fields(db)
        .into_iter()
        .map(|field| Member {
            ty: ty::generate(db, &field.ty(db)),
            name: field_name_to_identifier(&field.name(db).to_string()),
            bit_field_size: None,
        })
        .collect();

    Struct::Definition {
        name: Some(full_name_to_identifier(&ty.full_name(db))),
        members,
    }
}

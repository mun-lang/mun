use mun_hir_input::FileId;

use super::Struct;
use crate::{
    diagnostics::ExportedPrivate, resolve::HasResolver, visibility::RawVisibility, DiagnosticSink,
    HasVisibility, HirDatabase, Ty, Visibility,
};

#[cfg(test)]
mod tests;

pub struct StructValidator<'a> {
    strukt: Struct,
    db: &'a dyn HirDatabase,
    file_id: FileId,
}

impl<'a> StructValidator<'a> {
    pub fn new(strukt: Struct, db: &'a dyn HirDatabase, file_id: FileId) -> Self {
        StructValidator {
            strukt,
            db,
            file_id,
        }
    }

    pub fn validate_privacy(&self, sink: &mut DiagnosticSink<'_>) {
        let resolver = self.strukt.id.resolver(self.db.upcast());
        let struct_data = self.strukt.data(self.db.upcast());

        let public_fields = struct_data
            .fields
            .iter()
            .filter(|(_, field_data)| field_data.visibility == RawVisibility::Public);

        let field_types = public_fields.map(|(_, field_data)| {
            let type_ref = field_data.type_ref;
            let (ty, _) = Ty::from_hir(self.db, &resolver, struct_data.type_ref_map(), type_ref);
            (ty, type_ref)
        });

        let struct_visibility = self.strukt.visibility(self.db);
        let type_is_allowed = |ty: &Ty| match struct_visibility {
            Visibility::Module(module_id) => {
                ty.visibility(self.db).is_visible_from(self.db, module_id)
            }
            Visibility::Public => ty.visibility(self.db).is_externally_visible(),
        };

        field_types
            .filter(|(ty, _)| !type_is_allowed(ty))
            .for_each(|(_, type_ref)| {
                sink.push(ExportedPrivate {
                    file: self.file_id,
                    type_ref: struct_data
                        .type_ref_source_map()
                        .type_ref_syntax(type_ref)
                        .unwrap(),
                });
            });
    }
}

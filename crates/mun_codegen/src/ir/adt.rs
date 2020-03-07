//use crate::ir::module::Types;
use crate::ir::try_convert_any_to_basic;
use crate::{CodeGenParams, IrDatabase};
use inkwell::types::{BasicTypeEnum, StructType};

pub(super) fn gen_struct_decl(db: &impl IrDatabase, s: hir::Struct) -> StructType {
    let struct_type = db.struct_ty(s);
    if struct_type.is_opaque() {
        let field_types: Vec<BasicTypeEnum> = s
            .fields(db)
            .iter()
            .map(|field| {
                let field_type = field.ty(db);
                try_convert_any_to_basic(db.type_ir(
                    field_type,
                    CodeGenParams {
                        make_marshallable: false,
                    },
                ))
                .expect("could not convert field type")
            })
            .collect();

        struct_type.set_body(&field_types, false);
    }
    struct_type
}

//use crate::ir::module::Types;
use crate::ir::try_convert_any_to_basic;
use crate::IrDatabase;
use inkwell::types::{AnyTypeEnum, BasicTypeEnum, StructType};
use inkwell::values::BasicValueEnum;

pub(super) fn gen_struct_decl(db: &impl IrDatabase, s: hir::Struct) -> StructType {
    if let AnyTypeEnum::StructType(struct_type) = db.type_ir(s.ty(db)) {
        if struct_type.is_opaque() {
            let field_types: Vec<BasicTypeEnum> = s
                .fields(db)
                .iter()
                .map(|field| {
                    let field_type = field.ty(db);
                    try_convert_any_to_basic(db.type_ir(field_type))
                        .expect("could not convert field type")
                })
                .collect();

            struct_type.set_body(&field_types, false);
        }
        struct_type
    } else {
        unreachable!()
    }
}

pub(crate) fn gen_named_struct_lit(
    db: &impl IrDatabase,
    ty: hir::Ty,
    values: &[BasicValueEnum],
) -> BasicValueEnum {
    if let inkwell::types::AnyTypeEnum::StructType(struct_type) = db.type_ir(ty) {
        struct_type.const_named_struct(&values).into()
    } else {
        unreachable!("at this stage there must be a struct type");
    }
}

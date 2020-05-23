//use crate::ir::module::Types;
use inkwell::context::Context;
use crate::ir::ty::TypeManager;
use crate::ir::try_convert_any_to_basic;
use crate::{CodeGenParams};
use inkwell::types::{BasicTypeEnum, StructType};

pub(super) fn gen_struct_decl(context: &Context, db: &impl hir::HirDatabase, type_manager: &mut TypeManager, s: hir::Struct) -> StructType {
    let struct_type = type_manager.struct_ty(context, db, s);
    if struct_type.is_opaque() {
        let field_types: Vec<BasicTypeEnum> = s
            .fields(db)
            .iter()
            .map(|field| {
                let field_type = field.ty(db);
                try_convert_any_to_basic(type_manager.type_ir(
                    context,
                    db,
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

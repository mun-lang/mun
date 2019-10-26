use super::try_convert_any_to_basic;
use crate::IrDatabase;
use inkwell::types::{AnyTypeEnum, BasicType, BasicTypeEnum};
use mun_hir::{ApplicationTy, Ty, TypeCtor};

/// Given a mun type, construct an LLVM IR type
pub(crate) fn ir_query(db: &impl IrDatabase, ty: Ty) -> AnyTypeEnum {
    let context = db.context();
    match ty {
        Ty::Empty => AnyTypeEnum::VoidType(context.void_type()),
        Ty::Apply(ApplicationTy { ctor, .. }) => match ctor {
            TypeCtor::Float => AnyTypeEnum::FloatType(context.f64_type()),
            TypeCtor::Int => AnyTypeEnum::IntType(context.i64_type()),
            TypeCtor::Bool => AnyTypeEnum::IntType(context.bool_type()),
            TypeCtor::FnDef(f) => {
                let ty = db.fn_signature(f);
                let params: Vec<BasicTypeEnum> = ty
                    .params()
                    .iter()
                    .map(|p| try_convert_any_to_basic(db.type_ir(p.clone())).unwrap())
                    .collect();
                let ret_ty = match db.type_ir(ty.ret().clone()) {
                    AnyTypeEnum::VoidType(v) => return v.fn_type(&params, false).into(),
                    v => try_convert_any_to_basic(v).expect("could not convert return value"),
                };

                ret_ty.fn_type(&params, false).into()
            }
            _ => unreachable!(),
        },
        _ => unreachable!("unknown type can not be converted"),
    }
}

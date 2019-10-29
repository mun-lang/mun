use inkwell::types::{AnyTypeEnum, BasicTypeEnum};

pub mod body;
pub(crate) mod dispatch_table;
pub mod function;
pub mod module;
pub mod ty;

/// Try to down cast an `AnyTypeEnum` into a `BasicTypeEnum`.
fn try_convert_any_to_basic(ty: AnyTypeEnum) -> Option<BasicTypeEnum> {
    match ty {
        AnyTypeEnum::ArrayType(t) => Some(t.into()),
        AnyTypeEnum::FloatType(t) => Some(t.into()),
        AnyTypeEnum::IntType(t) => Some(t.into()),
        AnyTypeEnum::PointerType(t) => Some(t.into()),
        AnyTypeEnum::StructType(t) => Some(t.into()),
        AnyTypeEnum::VectorType(t) => Some(t.into()),
        _ => None,
    }
}

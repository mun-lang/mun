use super::try_convert_any_to_basic;
use crate::{
    type_info::{TypeGroup, TypeInfo},
    CodeGenParams, IrDatabase,
};
use hir::{ApplicationTy, CallableDef, FloatBitness, FloatTy, IntBitness, IntTy, Ty, TypeCtor};
use inkwell::types::{AnyTypeEnum, BasicType, BasicTypeEnum, StructType};
use inkwell::AddressSpace;

/// Given a mun type, construct an LLVM IR type
#[rustfmt::skip]
pub(crate) fn ir_query(db: &impl IrDatabase, ty: Ty, params: CodeGenParams) -> AnyTypeEnum {
    let context = db.context();
    match ty {
        Ty::Empty => AnyTypeEnum::StructType(context.struct_type(&[], false)),
        Ty::Apply(ApplicationTy { ctor, .. }) => match ctor {
            // Float primitives
            TypeCtor::Float(FloatTy { bitness: FloatBitness::X32 })       => AnyTypeEnum::FloatType(context.f32_type()),
            TypeCtor::Float(FloatTy { bitness: FloatBitness::X64 })       => AnyTypeEnum::FloatType(context.f64_type()),
            TypeCtor::Float(FloatTy { bitness: FloatBitness::Undefined }) => AnyTypeEnum::FloatType(context.f64_type()),

            // Int primitives
            TypeCtor::Int(IntTy { bitness: IntBitness::Undefined, .. })   => AnyTypeEnum::IntType(context.i64_type()),
            TypeCtor::Int(IntTy { bitness: IntBitness::X8, .. })          => AnyTypeEnum::IntType(context.i8_type()),
            TypeCtor::Int(IntTy { bitness: IntBitness::X16, .. })         => AnyTypeEnum::IntType(context.i16_type()),
            TypeCtor::Int(IntTy { bitness: IntBitness::X32, .. })         => AnyTypeEnum::IntType(context.i32_type()),
            TypeCtor::Int(IntTy { bitness: IntBitness::X64, .. })         => AnyTypeEnum::IntType(context.i64_type()),
            TypeCtor::Int(IntTy { bitness: IntBitness::Xsize, .. })       => AnyTypeEnum::IntType(context.i64_type()),

            // Boolean
            TypeCtor::Bool                                                => AnyTypeEnum::IntType(context.bool_type()),

            TypeCtor::FnDef(def @ CallableDef::Function(_)) => {
                let ty = db.callable_sig(def);
                let param_tys: Vec<BasicTypeEnum> = ty
                    .params()
                    .iter()
                    .map(|p| {
                        try_convert_any_to_basic(db.type_ir(p.clone(), params.clone())).unwrap()
                    })
                    .collect();

                let fn_type = match ty.ret() {
                    Ty::Empty => context.void_type().fn_type(&param_tys, false),
                    ty => try_convert_any_to_basic(db.type_ir(ty.clone(), params))
                        .expect("could not convert return value")
                        .fn_type(&param_tys, false),
                };

                AnyTypeEnum::FunctionType(fn_type)
            }
            TypeCtor::Struct(s) => {
                let struct_ty = db.struct_ty(s);
                match s.data(db).memory_kind {
                    hir::StructMemoryKind::GC => struct_ty.ptr_type(AddressSpace::Generic).into(),
                    hir::StructMemoryKind::Value => {
                        if params.is_extern {
                            struct_ty.ptr_type(AddressSpace::Generic).into()
                        } else {
                            struct_ty.into()
                        }
                    }
                }
            }
            _ => unreachable!(),
        },
        _ => unreachable!("unknown type can not be converted"),
    }
}

/// Returns the LLVM IR type of the specified struct
pub fn struct_ty_query(db: &impl IrDatabase, s: hir::Struct) -> StructType {
    let name = s.name(db).to_string();
    for field in s.fields(db).iter() {
        // Ensure that salsa's cached value incorporates the struct fields
        let _field_type_ir = db.type_ir(field.ty(db), CodeGenParams { is_extern: false });
    }

    db.context().opaque_struct_type(&name)
}

/// Constructs the `TypeInfo` for the specified HIR type
pub fn type_info_query(db: &impl IrDatabase, ty: Ty) -> TypeInfo {
    match ty {
        Ty::Apply(ctor) => match ctor.ctor {
            TypeCtor::Float(ty) => {
                TypeInfo::new(format!("core::{}", ty), TypeGroup::FundamentalTypes)
            }
            TypeCtor::Int(ty) => {
                TypeInfo::new(format!("core::{}", ty), TypeGroup::FundamentalTypes)
            }
            TypeCtor::Bool => TypeInfo::new("core::bool", TypeGroup::FundamentalTypes),
            TypeCtor::Struct(s) => TypeInfo::new(s.name(db).to_string(), TypeGroup::StructTypes(s)),
            _ => unreachable!("{:?} unhandled", ctor),
        },
        _ => unreachable!("{:?} unhandled", ty),
    }
}

use super::try_convert_any_to_basic;
use crate::type_info::TypeSize;
use crate::{
    type_info::{TypeGroup, TypeInfo},
    CodeGenParams, IrDatabase,
};
use hir::{ApplicationTy, CallableDef, FloatBitness, FloatTy, IntBitness, IntTy, Ty, TypeCtor};
use inkwell::targets::TargetData;
use inkwell::types::{AnyTypeEnum, BasicType, BasicTypeEnum, FloatType, IntType, StructType};
use inkwell::AddressSpace;

/// Given a mun type, construct an LLVM IR type
#[rustfmt::skip]
pub(crate) fn ir_query(db: &impl IrDatabase, ty: Ty, params: CodeGenParams) -> AnyTypeEnum {
    let context = db.context();
    match ty {
        Ty::Empty => AnyTypeEnum::StructType(context.struct_type(&[], false)),
        Ty::Apply(ApplicationTy { ctor, .. }) => match ctor {
            TypeCtor::Float(fty) => float_ty_query(db, fty).into(),
            TypeCtor::Int(ity) => int_ty_query(db, ity).into(),
            TypeCtor::Bool => AnyTypeEnum::IntType(context.bool_type()),

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
                    hir::StructMemoryKind::GC => struct_ty.ptr_type(AddressSpace::Generic).ptr_type(AddressSpace::Const).into(),
                    hir::StructMemoryKind::Value if params.make_marshallable =>
                            struct_ty.ptr_type(AddressSpace::Generic).ptr_type(AddressSpace::Const).into(),
                    hir::StructMemoryKind::Value => struct_ty.into(),
                }
            }
            _ => unreachable!(),
        },
        _ => unreachable!("unknown type can not be converted"),
    }
}

/// Returns the LLVM IR type of the specified float type
fn float_ty_query(db: &impl IrDatabase, fty: FloatTy) -> FloatType {
    let context = db.context();
    match fty.resolve(&db.target_data()).bitness {
        FloatBitness::X64 => context.f64_type(),
        FloatBitness::X32 => context.f32_type(),
        _ => unreachable!(),
    }
}

/// Returns the LLVM IR type of the specified int type
fn int_ty_query(db: &impl IrDatabase, ity: IntTy) -> IntType {
    let context = db.context();
    match ity.resolve(&db.target_data()).bitness {
        IntBitness::X128 => context.i128_type(),
        IntBitness::X64 => context.i64_type(),
        IntBitness::X32 => context.i32_type(),
        IntBitness::X16 => context.i16_type(),
        IntBitness::X8 => context.i8_type(),
        _ => unreachable!(),
    }
}

/// Returns the LLVM IR type of the specified struct
pub fn struct_ty_query(db: &impl IrDatabase, s: hir::Struct) -> StructType {
    let name = s.name(db).to_string();
    for field in s.fields(db).iter() {
        // Ensure that salsa's cached value incorporates the struct fields
        let _field_type_ir = db.type_ir(
            field.ty(db),
            CodeGenParams {
                make_marshallable: false,
            },
        );
    }

    db.context().opaque_struct_type(&name)
}

/// Constructs the `TypeInfo` for the specified HIR type
pub fn type_info_query(db: &impl IrDatabase, ty: Ty) -> TypeInfo {
    let target = db.target_data();
    match ty {
        Ty::Apply(ctor) => match ctor.ctor {
            TypeCtor::Float(ty) => {
                let ir_ty = float_ty_query(db, ty);
                let type_size = TypeSize::from_ir_type(&ir_ty, target.as_ref());
                TypeInfo::new(
                    format!("core::{}", ty.resolve(&db.target_data())),
                    TypeGroup::FundamentalTypes,
                    type_size,
                )
            }
            TypeCtor::Int(ty) => {
                let ir_ty = int_ty_query(db, ty);
                let type_size = TypeSize::from_ir_type(&ir_ty, target.as_ref());
                TypeInfo::new(
                    format!("core::{}", ty.resolve(&db.target_data())),
                    TypeGroup::FundamentalTypes,
                    type_size,
                )
            }
            TypeCtor::Bool => {
                let ir_ty = db.context().bool_type();
                let type_size = TypeSize::from_ir_type(&ir_ty, target.as_ref());
                TypeInfo::new("core::bool", TypeGroup::FundamentalTypes, type_size)
            }
            TypeCtor::Struct(s) => {
                let ir_ty = db.struct_ty(s);
                let type_size = TypeSize::from_ir_type(&ir_ty, target.as_ref());
                TypeInfo::new(s.name(db).to_string(), TypeGroup::StructTypes(s), type_size)
            }
            _ => unreachable!("{:?} unhandled", ctor),
        },
        _ => unreachable!("{:?} unhandled", ty),
    }
}

pub(crate) trait ResolveBitness {
    fn resolve(&self, target: &TargetData) -> Self;
}

impl ResolveBitness for FloatTy {
    fn resolve(&self, _target: &TargetData) -> Self {
        let bitness = match self.bitness {
            FloatBitness::Undefined => FloatBitness::X64,
            bitness => bitness,
        };
        FloatTy { bitness }
    }
}

impl ResolveBitness for IntTy {
    fn resolve(&self, target: &TargetData) -> Self {
        let ptr_bit_size = target.ptr_sized_int_type(None).get_bit_width();
        let bitness = match ptr_bit_size {
            16 => IntBitness::X16,
            32 => IntBitness::X32,
            64 => IntBitness::X64,
            128 => IntBitness::X128,
            _ => unreachable!("unsupported bit size for pointers"),
        };
        let bitness = match self.bitness {
            IntBitness::Undefined => IntBitness::X64,
            IntBitness::Xsize => bitness,
            bitness => bitness,
        };
        IntTy {
            bitness,
            signedness: self.signedness,
        }
    }
}

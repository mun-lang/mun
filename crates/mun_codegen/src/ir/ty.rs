use std::collections::HashMap;
use mun_target::abi::TargetDataLayout;
use inkwell::context::Context;
use super::try_convert_any_to_basic;
use crate::{
    type_info::{TypeInfo, TypeSize},
    CodeGenParams, IrDatabase,
};
use hir::{
    ApplicationTy, CallableDef, FloatBitness, FloatTy, IntBitness, IntTy, ResolveBitness, Ty,
    TypeCtor,
};
use inkwell::{
    types::{AnyTypeEnum, BasicType, BasicTypeEnum, FloatType, IntType, StructType},
    AddressSpace,
};

/// Given a mun type, construct an LLVM IR type
#[rustfmt::skip]
pub(crate) fn ir_query(db: &impl IrDatabase, ty: Ty, params: CodeGenParams) -> AnyTypeEnum {
    let context = db.context();
    let layout = db.target_data_layout();
    match ty {
        Ty::Empty => AnyTypeEnum::StructType(context.struct_type(&[], false)),
        Ty::Apply(ApplicationTy { ctor, .. }) => match ctor {
            TypeCtor::Float(fty) => float_ty_query(context.as_ref(), &layout, fty).into(),
            TypeCtor::Int(ity) => int_ty_query(context.as_ref(), &layout, ity).into(),
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

#[derive(Debug)]
pub struct TypeManager {
    infos: HashMap<hir::Ty, TypeInfo>,
}

impl TypeManager {
    pub fn new() -> TypeManager {
        TypeManager {
            infos: HashMap::new(),
        }
    }

    pub fn type_info<D: IrDatabase>(&mut self, db: &D, ty: hir::Ty) -> TypeInfo {
        if let Some(info) = self.infos.get(&ty) {
            return info.clone();
        }

        let context = db.context();
        let target = db.target_data();
        let layout = db.target_data_layout();
 
        let res = match &ty {
            Ty::Apply(ctor) => match ctor.ctor {
                TypeCtor::Float(ty) => {
                    let ir_ty = float_ty_query(&context, &layout, ty);
                    let type_size = TypeSize::from_ir_type(&ir_ty, target.as_ref());
                    TypeInfo::new_fundamental(
                        format!("core::{}", ty.resolve(&layout)),
                        type_size,
                    )
                }
                TypeCtor::Int(ty) => {
                    let ir_ty = int_ty_query(&context, &layout, ty);
                    let type_size = TypeSize::from_ir_type(&ir_ty, target.as_ref());
                    TypeInfo::new_fundamental(
                        format!("core::{}", ty.resolve(&layout)),
                        type_size,
                    )
                }
                TypeCtor::Bool => {
                    let ir_ty = context.bool_type();
                    let type_size = TypeSize::from_ir_type(&ir_ty, target.as_ref());
                    TypeInfo::new_fundamental("core::bool", type_size)
                }
                TypeCtor::Struct(s) => {
                    let ir_ty = db.struct_ty(s);
                    let type_size = TypeSize::from_ir_type(&ir_ty, target.as_ref());
                    return TypeInfo::new_struct(db, s, type_size)
                }
                _ => unreachable!("{:?} unhandled", ctor),
            },
            _ => unreachable!("{:?} unhandled", ty),
        };

        assert!(self.infos.insert(ty, res.clone()).is_none());
        res
    }
}

/// Returns the LLVM IR type of the specified float type
fn float_ty_query(context: &Context, layout: &TargetDataLayout, fty: FloatTy) -> FloatType {
    match fty.bitness.resolve(layout) {
        FloatBitness::X64 => context.f64_type(),
        FloatBitness::X32 => context.f32_type(),
    }
}

/// Returns the LLVM IR type of the specified int type
fn int_ty_query(context: &Context, layout: &TargetDataLayout, ity: IntTy) -> IntType {
    match ity.bitness.resolve(layout) {
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

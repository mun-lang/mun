use c_codegen::{
    r#type::{OpaqueType, Pointer, Real, StrongInt},
    ConcreteType,
};
use mun_hir::{FloatBitness, IntBitness, Signedness, TyKind};

pub fn generate(ty: &mun_hir::Ty) -> ConcreteType {
    match ty.interned() {
        TyKind::Struct(_) => unimplemented!(),
        TyKind::Float(float_ty) => generate_float(float_ty),
        TyKind::Int(int_ty) => generate_int(int_ty),
        TyKind::Bool => unimplemented!(),
        TyKind::Tuple(0, _) => ConcreteType::Void,
        TyKind::Tuple(_, _substitution) => unimplemented!(),
        TyKind::InferenceVar(infer_ty) => unimplemented!(),
        TyKind::TypeAlias(type_alias) => unimplemented!(),
        TyKind::Never => unimplemented!(),
        TyKind::FnDef(callable_def, substitution) => unimplemented!(),
        TyKind::Array(ty) => unimplemented!(),
        TyKind::RawPtr(pointer_ty) => generate_pointer(pointer_ty),
        TyKind::Unknown => unimplemented!(),
    }
}

fn generate_float(float_ty: &mun_hir::FloatTy) -> ConcreteType {
    match float_ty.bitness {
        FloatBitness::X32 => Real::Float.into(),
        FloatBitness::X64 => Real::Double.into(),
    }
}

fn generate_int(int_ty: &mun_hir::IntTy) -> ConcreteType {
    match (int_ty.signedness, int_ty.bitness) {
        (Signedness::Signed, IntBitness::Xsize) => unimplemented!(),
        (Signedness::Signed, IntBitness::X8) => StrongInt::Int8.into(),
        (Signedness::Signed, IntBitness::X16) => StrongInt::Int16.into(),
        (Signedness::Signed, IntBitness::X32) => StrongInt::Int32.into(),
        (Signedness::Signed, IntBitness::X64) => StrongInt::Int64.into(),
        (Signedness::Signed, IntBitness::X128) => unimplemented!(),
        (Signedness::Unsigned, IntBitness::Xsize) => ConcreteType::Size,
        (Signedness::Unsigned, IntBitness::X8) => StrongInt::Uint8.into(),
        (Signedness::Unsigned, IntBitness::X16) => StrongInt::Uint16.into(),
        (Signedness::Unsigned, IntBitness::X32) => StrongInt::Uint32.into(),
        (Signedness::Unsigned, IntBitness::X64) => StrongInt::Uint64.into(),
        (Signedness::Unsigned, IntBitness::X128) => unimplemented!(),
    }
}

fn generate_pointer(pointer_ty: &mun_hir::PointerTy) -> ConcreteType {
    Pointer {
        pointer_ty: OpaqueType::ConcreteType(generate(&pointer_ty.pointee_ty)),
        is_const: pointer_ty.mutability.is_const(),
    }
    .into()
}

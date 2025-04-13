use std::sync::OnceLock;

use mun_abi::static_type_map::StaticTypeMap;
use mun_hir::{FloatTy, IntTy, Mutability, PointerTy, Ty, TyKind};

pub trait HasStaticType {
    fn ty() -> &'static Ty;
}

macro_rules! impl_primitive_type {
    ($(
        $ty:ty => $kind:expr
    ),+) => {
        $(
            impl HasStaticType for $ty {
                fn ty() -> &'static Ty {
                    static TYPE_INFO: once_cell::sync::OnceCell<Ty> = once_cell::sync::OnceCell::new();
                    TYPE_INFO.get_or_init(|| {
                        $kind.intern()
                    })
                }
            }
        )+
    }
}

impl_primitive_type! {
    i8 => TyKind::Int(IntTy::i8()),
    i16 => TyKind::Int(IntTy::i16()),
    i32 => TyKind::Int(IntTy::i32()),
    i64 => TyKind::Int(IntTy::i64()),
    i128 => TyKind::Int(IntTy::i128()),
    isize => TyKind::Int(IntTy::isize()),
    u8 => TyKind::Int(IntTy::u8()),
    u16 => TyKind::Int(IntTy::u16()),
    u32 => TyKind::Int(IntTy::u32()),
    u64 => TyKind::Int(IntTy::u64()),
    u128 => TyKind::Int(IntTy::u128()),
    usize => TyKind::Int(IntTy::usize()),
    f32 => TyKind::Float(FloatTy::f32()),
    f64 => TyKind::Float(FloatTy::f64()),
    bool => TyKind::Bool,
    () => TyKind::unit(),
    std::ffi::c_void => TyKind::unit()
}

impl<T: HasStaticType + 'static> HasStaticType for *const T {
    fn ty() -> &'static Ty {
        static INIT: OnceLock<StaticTypeMap<Ty>> = OnceLock::new();
        INIT.get_or_init(StaticTypeMap::default)
            .call_once::<T, _>(|| {
                TyKind::RawPtr(PointerTy {
                    pointee_ty: T::ty().clone(),
                    mutability: Mutability::Shared,
                })
                .intern()
            })
    }
}

impl<T: HasStaticType + 'static> HasStaticType for *mut T {
    fn ty() -> &'static Ty {
        static INIT: OnceLock<StaticTypeMap<Ty>> = OnceLock::new();
        INIT.get_or_init(StaticTypeMap::default)
            .call_once::<T, _>(|| {
                TyKind::RawPtr(PointerTy {
                    pointee_ty: T::ty().clone(),
                    mutability: Mutability::Mut,
                })
                .intern()
            })
    }
}

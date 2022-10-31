use crate::{r#type::ffi::Type, HasStaticType};

/// Types of primitives supported by Mun.
#[repr(u8)]
#[derive(Clone, Copy)]
#[allow(missing_docs)]
pub enum PrimitiveType {
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    F32,
    F64,
    Empty,
    Void,
}

/// Returns a [`Type`] that represents the specified primitive type.
#[no_mangle]
pub extern "C" fn mun_type_primitive(primitive_type: PrimitiveType) -> Type {
    match primitive_type {
        PrimitiveType::Bool => bool::type_info(),
        PrimitiveType::U8 => u8::type_info(),
        PrimitiveType::U16 => u16::type_info(),
        PrimitiveType::U32 => u32::type_info(),
        PrimitiveType::U64 => u64::type_info(),
        PrimitiveType::U128 => u128::type_info(),
        PrimitiveType::I8 => i8::type_info(),
        PrimitiveType::I16 => i16::type_info(),
        PrimitiveType::I32 => i32::type_info(),
        PrimitiveType::I64 => i64::type_info(),
        PrimitiveType::I128 => i128::type_info(),
        PrimitiveType::F32 => f32::type_info(),
        PrimitiveType::F64 => f64::type_info(),
        PrimitiveType::Empty => <()>::type_info(),
        PrimitiveType::Void => <std::ffi::c_void>::type_info(),
    }
    .clone()
    .into()
}

#[cfg(test)]
mod test {
    use super::{
        super::{mun_type_kind, TypeKind},
        mun_type_primitive,
        PrimitiveType::{self, *},
    };
    use crate::HasStaticType;
    use mun_capi_utils::assert_getter1;

    #[test]
    fn test_primitives() {
        test_primitive::<bool>(Bool);
        test_primitive::<u8>(U8);
        test_primitive::<u16>(U16);
        test_primitive::<u32>(U32);
        test_primitive::<u64>(U64);
        test_primitive::<u128>(U128);
        test_primitive::<i8>(I8);
        test_primitive::<i16>(I16);
        test_primitive::<i32>(I32);
        test_primitive::<i64>(I64);
        test_primitive::<i128>(I128);
        test_primitive::<f32>(F32);
        test_primitive::<f64>(F64);
        test_primitive::<()>(Empty);
        test_primitive::<std::ffi::c_void>(Void);

        fn test_primitive<T: HasStaticType>(primitive_type: PrimitiveType) {
            let ffi_ty = mun_type_primitive(primitive_type);

            assert_getter1!(mun_type_kind(ffi_ty, ffi_kind));
            let guid = match ffi_kind {
                TypeKind::Primitive(guid) => guid,
                _ => panic!("invalid type kind for primitive"),
            };

            let rust_ty = unsafe { ffi_ty.to_owned() }.unwrap();
            let static_ty = T::type_info();
            assert_eq!(&rust_ty, static_ty);
            assert_eq!(static_ty.as_concrete().unwrap(), &guid);
        }
    }
}

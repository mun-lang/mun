use crate::{
    FunctionDefinition, FunctionPrototype, FunctionSignature, HasStaticTypeInfo, TypeInfo,
};
use std::{ffi::CString, ptr};

/// Owned storage for C-style `FunctionDefinition`.
pub struct FunctionDefinitionStorage {
    _name: CString,
    _type_infos: Vec<&'static TypeInfo>,
}

impl FunctionDefinitionStorage {
    /// Constructs a new `FunctionDefinition`, the data of which is stored in a
    /// `FunctionDefinitionStorage`.
    pub fn new_function(
        name: &str,
        args: &[&'static TypeInfo],
        ret: Option<&'static TypeInfo>,
        fn_ptr: *const std::ffi::c_void,
    ) -> (FunctionDefinition, FunctionDefinitionStorage) {
        let name = CString::new(name).unwrap();
        let type_infos: Vec<&'static TypeInfo> = args.iter().copied().collect();

        let num_arg_types = type_infos.len() as u16;
        let return_type = if let Some(ty) = ret {
            ty as *const _
        } else {
            ptr::null()
        };

        let fn_info = FunctionDefinition {
            prototype: FunctionPrototype {
                name: name.as_ptr(),
                signature: FunctionSignature {
                    arg_types: type_infos.as_ptr() as *const *const _,
                    return_type,
                    num_arg_types,
                },
            },
            fn_ptr,
        };

        let fn_storage = FunctionDefinitionStorage {
            _name: name,
            _type_infos: type_infos,
        };

        (fn_info, fn_storage)
    }
}

/// A value-to-`FunctionDefinition` conversion that consumes the input value.
pub trait IntoFunctionDefinition {
    /// Performs the conversion.
    fn into<S: AsRef<str>>(self, name: S) -> (FunctionDefinition, FunctionDefinitionStorage);
}

macro_rules! into_function_info_impl {
    ($(
        extern "C" fn($($T:ident),*) -> $R:ident;
    )+) => {
        $(
            impl<$R: HasStaticTypeInfo, $($T: HasStaticTypeInfo,)*> IntoFunctionDefinition
            for extern "C" fn($($T),*) -> $R
            {
                fn into<S: AsRef<str>>(self, name: S) -> (FunctionDefinition, FunctionDefinitionStorage) {
                    FunctionDefinitionStorage::new_function(
                        name.as_ref(),
                        &[$($T::type_info(),)*],
                        Some($R::type_info()),
                        self as *const std::ffi::c_void,
                    )
                }
            }

            impl<$($T: HasStaticTypeInfo,)*> IntoFunctionDefinition
            for extern "C" fn($($T),*)
            {
                fn into<S: AsRef<str>>(self, name: S) -> (FunctionDefinition, FunctionDefinitionStorage) {
                    FunctionDefinitionStorage::new_function(
                        name.as_ref(),
                        &[$($T::type_info(),)*],
                        None,
                        self as *const std::ffi::c_void,
                    )
                }
            }
        )+
    }
}

into_function_info_impl! {
    extern "C" fn() -> R;
    extern "C" fn(A) -> R;
    extern "C" fn(A, B) -> R;
    extern "C" fn(A, B, C) -> R;
    extern "C" fn(A, B, C, D) -> R;
    extern "C" fn(A, B, C, D, E) -> R;
    extern "C" fn(A, B, C, D, E, F) -> R;
    extern "C" fn(A, B, C, D, E, F, G) -> R;
    extern "C" fn(A, B, C, D, E, F, G, H) -> R;
    extern "C" fn(A, B, C, D, E, F, G, H, I) -> R;
    extern "C" fn(A, B, C, D, E, F, G, H, I, J) -> R;
}

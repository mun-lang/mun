use std::ffi::CString;
use std::ptr;

use crate::type_info::HasStaticTypeInfo;

pub struct FunctionInfoStorage {
    _name: CString,
    _type_infos: Vec<&'static abi::TypeInfo>,
}

impl FunctionInfoStorage {
    pub fn new_function(
        name: &str,
        args: &[&'static abi::TypeInfo],
        ret: Option<&'static abi::TypeInfo>,
        privacy: abi::Privacy,
        fn_ptr: *const std::ffi::c_void,
    ) -> (abi::FunctionInfo, FunctionInfoStorage) {
        let name = CString::new(name).unwrap();
        let type_infos: Vec<&'static abi::TypeInfo> = args.iter().copied().collect();

        let num_arg_types = type_infos.len() as u16;
        let return_type = if let Some(ty) = ret {
            ty as *const _
        } else {
            ptr::null()
        };

        let fn_info = abi::FunctionInfo {
            signature: abi::FunctionSignature {
                name: name.as_ptr(),
                arg_types: type_infos.as_ptr() as *const *const _,
                return_type,
                num_arg_types,
                privacy,
            },
            fn_ptr,
        };

        let fn_storage = FunctionInfoStorage {
            _name: name,
            _type_infos: type_infos,
        };

        (fn_info, fn_storage)
    }
}

/// A value-to-`FunctionInfo` conversion that consumes the input value.
pub trait IntoFunctionInfo {
    /// Performs the conversion.
    fn into<S: AsRef<str>>(
        self,
        name: S,
        privacy: abi::Privacy,
    ) -> (abi::FunctionInfo, FunctionInfoStorage);
}

macro_rules! into_function_info_impl {
    ($(
        extern "C" fn($($T:ident),*) -> $R:ident;
    )+) => {
        $(
            impl<$R: HasStaticTypeInfo, $($T: HasStaticTypeInfo,)*> IntoFunctionInfo
            for extern "C" fn($($T),*) -> $R
            {
                fn into<S: AsRef<str>>(self, name: S, privacy: abi::Privacy) -> (abi::FunctionInfo, FunctionInfoStorage) {
                    FunctionInfoStorage::new_function(
                        name.as_ref(),
                        &[$($T::type_info(),)*],
                        Some($R::type_info()),
                        privacy,
                        self as *const std::ffi::c_void,
                    )
                }
            }

            impl<$($T: HasStaticTypeInfo,)*> IntoFunctionInfo
            for extern "C" fn($($T),*)
            {
                fn into<S: AsRef<str>>(self, name: S, privacy: abi::Privacy) -> (abi::FunctionInfo, FunctionInfoStorage) {
                    FunctionInfoStorage::new_function(
                        name.as_ref(),
                        &[$($T::type_info(),)*],
                        None,
                        privacy,
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

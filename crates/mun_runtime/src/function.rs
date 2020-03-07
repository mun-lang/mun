use std::ffi::CString;
use std::ptr;

use crate::ReturnTypeReflection;
use abi::{FunctionInfo, FunctionSignature, Guid, Privacy, TypeGroup, TypeInfo};

pub struct FunctionInfoStorage {
    _name: CString,
    _type_names: Vec<CString>,

    // Clippy warns: `Vec<T>` is already on the heap, the boxing is unnecessary.
    // However, in this case we explicitly want to have a Vec<T> of pointers.
    #[allow(clippy::vec_box)]
    _type_infos: Vec<Box<TypeInfo>>,
}

impl FunctionInfoStorage {
    pub fn new_function(
        name: &str,
        args: &[String],
        ret: Option<String>,
        privacy: Privacy,
        fn_ptr: *const std::ffi::c_void,
    ) -> (FunctionInfo, FunctionInfoStorage) {
        let name = CString::new(name).unwrap();
        let (mut type_names, mut type_infos): (Vec<CString>, Vec<Box<TypeInfo>>) = args
            .iter()
            .cloned()
            .map(|name| {
                let name = CString::new(name).unwrap();
                let type_info = Box::new(TypeInfo {
                    guid: Guid {
                        b: md5::compute(name.as_bytes()).0,
                    },
                    name: name.as_ptr(),
                    group: TypeGroup::FundamentalTypes,
                });
                (name, type_info)
            })
            .unzip();

        let ret = ret.map(|name| {
            let name = CString::new(name).unwrap();
            let type_info = Box::new(TypeInfo {
                guid: Guid {
                    b: md5::compute(name.as_bytes()).0,
                },
                name: name.as_ptr(),
                group: TypeGroup::FundamentalTypes,
            });
            (name, type_info)
        });

        let num_arg_types = type_infos.len() as u16;
        let return_type = if let Some((type_name, type_info)) = ret {
            type_names.push(type_name);

            let ptr = Box::into_raw(type_info);
            let type_info = unsafe { Box::from_raw(ptr) };
            type_infos.push(type_info);

            ptr
        } else {
            ptr::null()
        };

        let fn_info = FunctionInfo {
            signature: FunctionSignature {
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
            _type_names: type_names,
            _type_infos: type_infos,
        };

        (fn_info, fn_storage)
    }
}

pub trait IntoFunctionInfo {
    fn into<S: AsRef<str>>(
        self,
        name: S,
        privacy: abi::Privacy,
    ) -> (FunctionInfo, FunctionInfoStorage);
}

macro_rules! into_function_info_impl {
    ($(
        extern "C" fn($($T:ident),*) -> $R:ident;
    )+) => {
        $(
            impl<$R: ReturnTypeReflection, $($T: ReturnTypeReflection,)*> IntoFunctionInfo
            for extern "C" fn($($T),*) -> $R
            {
                fn into<S: AsRef<str>>(self, name: S, privacy: Privacy) -> (FunctionInfo, FunctionInfoStorage) {
                    FunctionInfoStorage::new_function(
                        name.as_ref(),
                        &[$($T::type_name().to_string(),)*],
                        Some($R::type_name().to_string()),
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

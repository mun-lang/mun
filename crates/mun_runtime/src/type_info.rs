// use once_cell::unsync::OnceCell;
// use std::ffi::{CStr, CString};
//
// /// A trait that defines that for a type we can statically return a abi::TypeInfo
// trait HasStaticTypeInfo {
//     /// Returns a reference to the TypeInfo for the type
//     fn type_info() -> &'static abi::TypeInfo;
// }
//
// /// A trait that defines that for a type we can statically return the name that would be used in a
// /// abi::TypeInfo. This is useful for opaque types that we do not know the full details of but we
// /// could use it as a pointer type
// trait HasStaticTypeInfoName {
//     /// Returns the type info name for the type
//     fn type_name() -> &'static CStr;
//
//     /// Retrieves the type's `Guid`.
//     fn type_guid() -> abi::Guid {
//         abi::Guid {
//             b: md5::compute(Self::type_name()).0,
//         }
//     }
// }
//
// /// Implement HasStaticTypeInfoName for everything that can produce a type info.
// impl<T: HasStaticTypeInfo> HasStaticTypeInfoName for T {
//     fn type_name() -> &'static CStr {
//         unsafe { CStr::from_ptr(Self::type_info().name) }
//     }
// }
//
// /// Every type that has at least a type name also has a valid pointer type name
// impl<T: HasStaticTypeInfoName> HasStaticTypeInfo for *const T {
//     fn type_info() -> &'static abi::TypeInfo {
//         let type_info: OnceCell<abi::TypeInfo> = OnceCell::new();
//         type_info.get_or_init(|| {
//             static TYPE_INFO_NAME: OnceCell<CString> = OnceCell::new();
//             let type_info_name: &'static CString = TYPE_INFO_NAME
//                 .get_or_init(|| CString::new(format!("*const {}", T::type_name().to_str().unwrap())).unwrap());
//
//             abi::TypeInfo {
//                 guid: abi::Guid{ b: md5::compute(&type_info_name.into_bytes()).0 },
//                 name: type_info_name.as_ptr(),
//                 group: abi::TypeGroup::FundamentalTypes
//             }
//         })
//     }
// }
//
// /// Every type that has at least a type name also has a valid pointer type name
// impl<T: HasStaticTypeInfoName> HasStaticTypeInfo for *mut T {
//     fn type_info() -> &'static abi::TypeInfo {
//         let type_info: OnceCell<abi::TypeInfo> = OnceCell::new();
//         type_info.get_or_init(|| {
//             static TYPE_INFO_NAME: OnceCell<CString> = OnceCell::new();
//             let type_info_name: &'static CString = TYPE_INFO_NAME
//                 .get_or_init(|| CString::new(format!("*const {}", T::type_name().to_str().unwrap())).unwrap());
//
//             abi::TypeInfo {
//                 guid: abi::Guid{ b: md5::compute(&type_info_name.into_bytes()).0 },
//                 name: type_info_name.as_ptr(),
//                 group: abi::TypeGroup::FundamentalTypes
//             }
//         })
//     }
// }
//
//

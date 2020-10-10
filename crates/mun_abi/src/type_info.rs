use crate::{static_type_map::StaticTypeMap, Guid, StructInfo};
use once_cell::sync::OnceCell;
use std::{
    convert::TryInto,
    ffi::{CStr, CString},
    fmt::{self, Formatter},
    mem,
    os::raw::c_char,
    str,
    sync::Once,
};

/// Represents the type declaration for a value type.
///
/// TODO: add support for polymorphism, enumerations, type parameters, generic type definitions, and
/// constructed generic types.
#[repr(C)]
#[derive(Debug)]
pub struct TypeInfo {
    /// Type GUID
    pub guid: Guid,
    /// Type name
    pub name: *const c_char,
    /// The exact size of the type in bits without any padding
    pub(crate) size_in_bits: u32,
    /// The alignment of the type
    pub(crate) alignment: u8,
    /// Type group
    pub group: TypeGroup,
}

/// Represents a group of types that illicit the same characteristics.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum TypeGroup {
    /// Fundamental types (i.e. `()`, `bool`, `float`, `int`, etc.)
    FundamentalTypes = 0,
    /// Struct types (i.e. record, tuple, or unit structs)
    StructTypes = 1,
}

impl TypeInfo {
    /// Returns the type's name.
    pub fn name(&self) -> &str {
        unsafe { str::from_utf8_unchecked(CStr::from_ptr(self.name).to_bytes()) }
    }

    /// Retrieves the type's struct information, if available.
    pub fn as_struct(&self) -> Option<&StructInfo> {
        if self.group.is_struct() {
            let ptr = (self as *const TypeInfo).cast::<u8>();
            let ptr = ptr.wrapping_add(mem::size_of::<TypeInfo>());
            let offset = ptr.align_offset(mem::align_of::<StructInfo>());
            let ptr = ptr.wrapping_add(offset);
            Some(unsafe { &*ptr.cast::<StructInfo>() })
        } else {
            None
        }
    }

    /// Returns the size of the type in bits
    pub fn size_in_bits(&self) -> usize {
        self.size_in_bits
            .try_into()
            .expect("cannot convert size in bits to platform size")
    }

    /// Returns the size of the type in bytes
    pub fn size_in_bytes(&self) -> usize {
        ((self.size_in_bits + 7) / 8)
            .try_into()
            .expect("cannot covert size in bytes to platform size")
    }

    /// Returns the alignment of the type in bytes
    pub fn alignment(&self) -> usize {
        self.alignment
            .try_into()
            .expect("cannot convert alignment to platform size")
    }
}

impl fmt::Display for TypeInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl PartialEq for TypeInfo {
    fn eq(&self, other: &Self) -> bool {
        self.guid == other.guid
    }
}

impl std::hash::Hash for TypeInfo {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.guid.hash(state);
    }
}

unsafe impl Send for TypeInfo {}
unsafe impl Sync for TypeInfo {}

impl TypeGroup {
    /// Returns whether this is a fundamental type.
    pub fn is_fundamental(self) -> bool {
        self == TypeGroup::FundamentalTypes
    }

    /// Returns whether this is a struct type.
    pub fn is_struct(self) -> bool {
        self == TypeGroup::StructTypes
    }
}

/// A trait that defines that for a type we can statically return a `TypeInfo`.
pub trait HasStaticTypeInfo {
    /// Returns a reference to the TypeInfo for the type
    fn type_info() -> &'static TypeInfo;
}

/// A trait that defines that for a type we can statically return the name that would be used in a
/// `TypeInfo`. This is useful for opaque types that we do not know the full details of but we could
/// use it as a pointer type
pub trait HasStaticTypeInfoName {
    /// Returns the type info name for the type
    fn type_name() -> &'static CStr;
}

/// Implement HasStaticTypeInfoName for everything that can produce a type info.
impl<T: HasStaticTypeInfo> HasStaticTypeInfoName for T {
    fn type_name() -> &'static CStr {
        unsafe { CStr::from_ptr(Self::type_info().name) }
    }
}

/// Every type that has at least a type name also has a valid pointer type name
impl<T: HasStaticTypeInfoName + 'static> HasStaticTypeInfo for *const T {
    fn type_info() -> &'static TypeInfo {
        static mut VALUE: Option<StaticTypeMap<(CString, TypeInfo)>> = None;
        static INIT: Once = Once::new();

        let map = unsafe {
            INIT.call_once(|| {
                VALUE = Some(StaticTypeMap::new());
            });
            VALUE.as_ref().unwrap()
        };

        &map.call_once::<T, _>(|| {
            let name =
                CString::new(format!("*const {}", T::type_name().to_str().unwrap())).unwrap();
            let guid = Guid(md5::compute(&name.as_bytes()).0);
            let name_ptr = name.as_ptr();
            (
                name,
                TypeInfo {
                    guid,
                    name: name_ptr,
                    group: TypeGroup::FundamentalTypes,
                    size_in_bits: (std::mem::size_of::<*const T>() * 8)
                        .try_into()
                        .expect("size of T is larger than the maximum allowed ABI size. Please file a bug."),
                    alignment: (std::mem::align_of::<*const T>())
                        .try_into()
                        .expect("alignment of T is larger than the maximum allowed ABI size. Please file a bug."),
                },
            )
        })
        .1
    }
}

/// Every type that has at least a type name also has a valid pointer type name
impl<T: HasStaticTypeInfoName + 'static> HasStaticTypeInfo for *mut T {
    fn type_info() -> &'static TypeInfo {
        static mut VALUE: Option<StaticTypeMap<(CString, TypeInfo)>> = None;
        static INIT: Once = Once::new();

        let map = unsafe {
            INIT.call_once(|| {
                VALUE = Some(StaticTypeMap::new());
            });
            VALUE.as_ref().unwrap()
        };

        &map.call_once::<T, _>(|| {
            let name = CString::new(format!("*mut {}", T::type_name().to_str().unwrap())).unwrap();
            let guid = Guid(md5::compute(&name.as_bytes()).0);
            let name_ptr = name.as_ptr();
            (
                name,
                TypeInfo {
                    guid,
                    name: name_ptr,
                    group: TypeGroup::FundamentalTypes,
                    size_in_bits: (std::mem::size_of::<*const T>() * 8)
                        .try_into()
                        .expect("size of T is larger than the maximum allowed ABI size. Please file a bug."),
                    alignment: (std::mem::align_of::<*const T>())
                        .try_into()
                        .expect("alignment of T is larger than the maximum allowed ABI size. Please file a bug."),
                },
            )
        })
        .1
    }
}

macro_rules! impl_basic_type_info {
    ($(
        $ty:ty
    ),+) => {
        $(
            impl HasStaticTypeInfo for $ty {
                fn type_info() -> &'static TypeInfo {
                    static TYPE_INFO: OnceCell<TypeInfo> = OnceCell::new();
                    TYPE_INFO.get_or_init(|| {
                        static TYPE_INFO_NAME: OnceCell<CString> = OnceCell::new();
                        let type_info_name: &'static CString = TYPE_INFO_NAME
                            .get_or_init(|| CString::new(format!("core::{}", stringify!($ty))).unwrap());

                        TypeInfo {
                            guid: Guid(md5::compute(&type_info_name.as_bytes()).0),
                            name: type_info_name.as_ptr(),
                            group: TypeGroup::FundamentalTypes,
                            size_in_bits: (std::mem::size_of::<$ty>() * 8)
                                .try_into()
                                .expect("size of T is larger than the maximum allowed ABI size. Please file a bug."),
                            alignment: (std::mem::align_of::<$ty>())
                                .try_into()
                                .expect("alignment of T is larger than the maximum allowed ABI size. Please file a bug."),
                        }
                    })
                }
            }
        )+
    }
}

macro_rules! impl_has_type_info_name {
    ($(
        $ty:ty => $name:tt
    ),+) => {
        $(
            impl crate::type_info::HasStaticTypeInfoName for $ty {
                fn type_name() -> &'static std::ffi::CStr {
                    static TYPE_INFO_NAME: once_cell::sync::OnceCell<std::ffi::CString> = once_cell::sync::OnceCell::new();
                    let type_info_name: &'static std::ffi::CString = TYPE_INFO_NAME
                        .get_or_init(|| std::ffi::CString::new($name).unwrap());
                    type_info_name.as_ref()
                }
            }
        )+
    }
}

impl_basic_type_info!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, f32, f64, bool);

impl_has_type_info_name!(
    std::ffi::c_void => "core::void",
    TypeInfo => "TypeInfo"
);

#[cfg(target_pointer_width = "64")]
impl HasStaticTypeInfo for usize {
    fn type_info() -> &'static TypeInfo {
        u64::type_info()
    }
}

#[cfg(target_pointer_width = "64")]
impl HasStaticTypeInfo for isize {
    fn type_info() -> &'static TypeInfo {
        i64::type_info()
    }
}

#[cfg(target_pointer_width = "32")]
impl HasStaticTypeInfo for usize {
    fn type_info() -> &'static TypeInfo {
        u32::type_info()
    }
}

#[cfg(target_pointer_width = "32")]
impl HasStaticTypeInfo for isize {
    fn type_info() -> &'static TypeInfo {
        i32::type_info()
    }
}

#[cfg(test)]
mod tests {
    use super::{HasStaticTypeInfoName, TypeGroup};
    use crate::test_utils::{fake_type_info, FAKE_TYPE_NAME};
    use std::ffi::CString;

    #[test]
    fn test_type_info_name() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name, TypeGroup::FundamentalTypes, 1, 1);

        assert_eq!(type_info.name(), FAKE_TYPE_NAME);
    }

    #[test]
    fn test_type_info_size_alignment() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name, TypeGroup::FundamentalTypes, 24, 8);

        assert_eq!(type_info.size_in_bits(), 24);
        assert_eq!(type_info.size_in_bytes(), 3);
        assert_eq!(type_info.alignment(), 8);
    }

    #[test]
    fn test_type_info_group_fundamental() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_group = TypeGroup::FundamentalTypes;
        let type_info = fake_type_info(&type_name, type_group, 1, 1);

        assert_eq!(type_info.group, type_group);
        assert!(type_info.group.is_fundamental());
        assert!(!type_info.group.is_struct());
    }

    #[test]
    fn test_type_info_group_struct() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_group = TypeGroup::StructTypes;
        let type_info = fake_type_info(&type_name, type_group, 1, 1);

        assert_eq!(type_info.group, type_group);
        assert!(type_info.group.is_struct());
        assert!(!type_info.group.is_fundamental());
    }

    #[test]
    fn test_type_info_eq() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name, TypeGroup::FundamentalTypes, 1, 1);

        assert_eq!(type_info, type_info);
    }

    #[test]
    fn test_ptr() {
        let ty = <*const std::ffi::c_void>::type_name();
        assert_eq!(ty.to_str().unwrap(), "*const core::void");

        let ty = <*const *mut std::ffi::c_void>::type_name();
        assert_eq!(ty.to_str().unwrap(), "*const *mut core::void");

        let ty = <*const *const std::ffi::c_void>::type_name();
        assert_eq!(ty.to_str().unwrap(), "*const *const core::void");
    }
}

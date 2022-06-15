use std::fmt::Debug;
use std::{
    convert::TryInto,
    ffi::{CStr, CString},
    fmt::{self, Formatter},
    os::raw::c_char,
    str,
    sync::Once,
};

use once_cell::sync::OnceCell;

use crate::type_lut::PointerTypeId;
use crate::{static_type_map::StaticTypeMap, Guid, StructInfo, TypeId};

/// Represents the type declaration for a value type.
///
/// TODO: add support for polymorphism, enumerations, type parameters, generic type definitions, and
/// constructed generic types.
#[repr(C)]
pub struct TypeInfo {
    /// Type name
    pub name: *const c_char,
    /// The exact size of the type in bits without any padding
    pub(crate) size_in_bits: u32,
    /// The alignment of the type
    pub(crate) alignment: u8,
    /// Type group
    pub data: TypeInfoData,
}

impl Debug for TypeInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("TypeInfo")
            .field("name", &self.name())
            .field("size_in_bits", &self.size_in_bits)
            .field("alignment", &self.alignment)
            .field("data", &self.data)
            .finish()
    }
}

/// Contains data specific to a group of types that illicit the same characteristics.
#[repr(u8)]
#[derive(Debug, PartialEq, Eq)]
pub enum TypeInfoData {
    /// Primitive types (i.e. `()`, `bool`, `float`, `int`, etc.)
    Primitive(Guid),
    /// Struct types (i.e. record, tuple, or unit structs)
    Struct(StructInfo),
    /// Pointer to another type
    Pointer(PointerInfo),
}

/// Pointer type information
#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub struct PointerInfo {
    /// The type to which this pointer points.
    pub pointee: TypeId,
    /// Whether or not the pointed to value is mutable or not
    pub mutable: bool,
}

impl TypeInfo {
    /// Returns true if this instance is an instance of the given `TypeId`.
    pub fn is_instance_of(&self, type_id: &TypeId) -> bool {
        match (&self.data, type_id) {
            (TypeInfoData::Pointer(p1), TypeId::Pointer(p2)) => &p1.pointee == p2.pointee(),
            (TypeInfoData::Struct(s), TypeId::Concrete(guid)) => &s.guid == guid,
            (TypeInfoData::Primitive(guid), TypeId::Concrete(g)) => guid == g,
            _ => false,
        }
    }

    /// Returns the type's name.
    pub fn name(&self) -> &str {
        unsafe { str::from_utf8_unchecked(CStr::from_ptr(self.name).to_bytes()) }
    }

    /// Retrieves the type's struct information, if available.
    pub fn as_struct(&self) -> Option<&StructInfo> {
        if let TypeInfoData::Struct(s) = &self.data {
            Some(s)
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
        self.size_in_bits == other.size_in_bits
            && self.alignment == other.alignment
            && self.data == other.data
    }
}

impl Eq for TypeInfo {}

unsafe impl Send for TypeInfo {}
unsafe impl Sync for TypeInfo {}

impl TypeInfoData {
    /// Returns whether this is a fundamental type.
    pub fn is_primitive(&self) -> bool {
        matches!(self, TypeInfoData::Primitive(_))
    }

    /// Returns whether this is a struct type.
    pub fn is_struct(&self) -> bool {
        matches!(self, TypeInfoData::Struct(_))
    }

    /// Returns whether this is a pointer type.
    pub fn is_pointer(&self) -> bool {
        matches!(self, TypeInfoData::Pointer(_))
    }
}

/// A trait that defines that for a type we can statically return a `TypeInfo`.
pub trait HasStaticTypeInfo {
    /// Returns a reference to the TypeInfo for the type
    fn type_info() -> &'static TypeInfo;
}

/// A trait that defines that for a type we can statically return a `TypeId`.
pub trait HasStaticTypeId {
    /// Returns a reference to the TypeInfo for the type
    fn type_id() -> &'static TypeId;
}

/// A trait that defines that for a type we can statically return a type name.
pub trait HasStaticTypeName {
    /// Returns a reference to the TypeInfo for the type
    fn type_name() -> &'static CStr;
}

impl<T: HasStaticTypeInfo> HasStaticTypeName for T {
    fn type_name() -> &'static CStr {
        unsafe { CStr::from_ptr(T::type_info().name) }
    }
}

impl<T: HasStaticTypeId + 'static> HasStaticTypeId for *const T {
    fn type_id() -> &'static TypeId {
        static VALUE: OnceCell<StaticTypeMap<TypeId>> = OnceCell::new();
        let map = VALUE.get_or_init(Default::default);
        &map.call_once::<T, _>(|| {
            PointerTypeId {
                pointee: T::type_id(),
                mutable: false,
            }
            .into()
        })
    }
}

impl<T: HasStaticTypeId + 'static> HasStaticTypeId for *mut T {
    fn type_id() -> &'static TypeId {
        static VALUE: OnceCell<StaticTypeMap<TypeId>> = OnceCell::new();
        let map = VALUE.get_or_init(Default::default);
        &map.call_once::<T, _>(|| {
            PointerTypeId {
                pointee: T::type_id(),
                mutable: true,
            }
            .into()
        })
    }
}

/// Every type that has at least a type name also has a valid pointer type name
impl<T: HasStaticTypeId + HasStaticTypeName + 'static> HasStaticTypeInfo for *const T {
    fn type_info() -> &'static TypeInfo {
        static mut VALUE: Option<StaticTypeMap<(CString, TypeInfo)>> = None;
        static INIT: Once = Once::new();

        let map = unsafe {
            INIT.call_once(|| {
                VALUE = Some(StaticTypeMap::default());
            });
            VALUE.as_ref().unwrap()
        };

        &map.call_once::<T, _>(|| {
            let element_name = T::type_name();
            let element_type_id = T::type_id();
            let name = CString::new(format!("*const {}", element_name.to_str().expect("could not convert type name to utf-8 string"))).unwrap();
            let name_ptr = name.as_ptr();
            (
                name,
                TypeInfo {
                    name: name_ptr,
                    size_in_bits: (std::mem::size_of::<*const T>() * 8)
                        .try_into()
                        .expect("size of T is larger than the maximum allowed ABI size. Please file a bug."),
                    alignment: (std::mem::align_of::<*const T>())
                        .try_into()
                        .expect("alignment of T is larger than the maximum allowed ABI size. Please file a bug."),
                    data: TypeInfoData::Pointer(PointerInfo {
                        pointee: element_type_id.clone(),
                        mutable: false
                    }),
                },
            )
        })
        .1
    }
}

/// Every type that has at least a type name also has a valid pointer type name
impl<T: HasStaticTypeId + HasStaticTypeName + 'static> HasStaticTypeInfo for *mut T {
    fn type_info() -> &'static TypeInfo {
        static mut VALUE: Option<StaticTypeMap<(CString, TypeInfo)>> = None;
        static INIT: Once = Once::new();

        let map = unsafe {
            INIT.call_once(|| {
                VALUE = Some(StaticTypeMap::default());
            });
            VALUE.as_ref().unwrap()
        };

        &map.call_once::<T, _>(|| {
            let element_name = T::type_name();
            let element_type_id = T::type_id();
            let name = CString::new(format!("*mut {}", element_name.to_str().expect("could not convert type name to utf-8 string"))).unwrap();
            let name_ptr = name.as_ptr();
            (
                name,
                TypeInfo {
                    name: name_ptr,
                    size_in_bits: (std::mem::size_of::<*const T>() * 8)
                        .try_into()
                        .expect("size of T is larger than the maximum allowed ABI size. Please file a bug."),
                    alignment: (std::mem::align_of::<*const T>())
                        .try_into()
                        .expect("alignment of T is larger than the maximum allowed ABI size. Please file a bug."),
                    data: TypeInfoData::Pointer(PointerInfo {
                        pointee: element_type_id.clone(),
                        mutable: true
                    }),
                },
            )
        })
            .1
    }
}

macro_rules! impl_primitive_type_info {
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
                        let guid = Guid::from(type_info_name.as_bytes());

                        TypeInfo {
                            name: type_info_name.as_ptr(),
                            size_in_bits: (std::mem::size_of::<$ty>() * 8)
                                .try_into()
                                .expect("size of T is larger than the maximum allowed ABI size. Please file a bug."),
                            alignment: (std::mem::align_of::<$ty>())
                                .try_into()
                                .expect("alignment of T is larger than the maximum allowed ABI size. Please file a bug."),
                            data: TypeInfoData::Primitive(guid),
                        }
                    })
                }
            }

            impl HasStaticTypeId for $ty {
                fn type_id() -> &'static TypeId {
                    const TYPE_ID: TypeId = TypeId::Concrete(Guid::from_str(concat!("core::", stringify!($ty))));
                    &TYPE_ID
                }
            }
        )+
    }
}

impl HasStaticTypeId for std::ffi::c_void {
    fn type_id() -> &'static TypeId {
        const TYPE_ID: TypeId = TypeId::Concrete(Guid::from_str("core::void"));
        &TYPE_ID
    }
}

impl_primitive_type_info!(
    i8,
    i16,
    i32,
    i64,
    i128,
    u8,
    u16,
    u32,
    u64,
    u128,
    f32,
    f64,
    bool,
    ()
);

impl HasStaticTypeInfo for std::ffi::c_void {
    fn type_info() -> &'static TypeInfo {
        static TYPE_INFO: OnceCell<TypeInfo> = OnceCell::new();
        TYPE_INFO.get_or_init(|| {
            static TYPE_INFO_NAME: OnceCell<CString> = OnceCell::new();
            let type_info_name: &'static CString = TYPE_INFO_NAME
                .get_or_init(|| CString::new("core::void").unwrap());
            let guid = Guid::from(type_info_name.as_bytes());

            TypeInfo {
                name: type_info_name.as_ptr(),
                size_in_bits: (std::mem::size_of::<std::ffi::c_void>() * 8)
                .try_into()
                .expect("size of T is larger than the maximum allowed ABI size. Please file a bug."),
                alignment: (std::mem::align_of::<std::ffi::c_void>())
                .try_into()
                .expect("alignment of T is larger than the maximum allowed ABI size. Please file a bug."),
                data: TypeInfoData::Primitive(guid),
            }
        })
    }
}

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
    use std::ffi::CString;

    use crate::test_utils::{
        fake_primitive_type_info, fake_struct_info, fake_type_info, FAKE_TYPE_NAME,
    };
    use crate::HasStaticTypeInfo;

    use super::TypeInfoData;

    #[test]
    fn test_type_info_name() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let (type_info, _type_id) = fake_primitive_type_info(&type_name, 1, 1);

        assert_eq!(type_info.name(), FAKE_TYPE_NAME);
    }

    #[test]
    fn test_type_info_size_alignment() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let (type_info, _type_id) = fake_primitive_type_info(&type_name, 24, 8);

        assert_eq!(type_info.size_in_bits(), 24);
        assert_eq!(type_info.size_in_bytes(), 3);
        assert_eq!(type_info.alignment(), 8);
    }

    #[test]
    fn test_type_info_group_fundamental() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let (type_info, _type_id) = fake_primitive_type_info(&type_name, 1, 1);

        assert!(type_info.data.is_primitive());
        assert!(!type_info.data.is_struct());
    }

    #[test]
    fn test_type_info_group_struct() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");

        let field_names = &[];
        let field_types = &[];
        let field_offsets = &[];
        let struct_info = fake_struct_info(
            &type_name,
            field_names,
            field_types,
            field_offsets,
            Default::default(),
        );

        let type_info = fake_type_info(&type_name, 1, 1, TypeInfoData::Struct(struct_info));

        assert!(type_info.data.is_struct());
        assert!(!type_info.data.is_primitive());
    }

    #[test]
    fn test_type_info_eq() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let (type_info, _type_id) = fake_primitive_type_info(&type_name, 1, 1);

        assert_eq!(type_info, type_info);
    }

    #[test]
    fn test_ptr() {
        let ty = <*const std::ffi::c_void>::type_info();
        assert_eq!(ty.name(), "*const core::void");

        let ty = <*const *mut std::ffi::c_void>::type_info();
        assert_eq!(ty.name(), "*const *mut core::void");

        let ty = <*const *const std::ffi::c_void>::type_info();
        assert_eq!(ty.name(), "*const *const core::void");
    }
}

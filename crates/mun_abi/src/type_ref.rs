use crate::{Guid, HasStaticTypeInfo, StructMemoryKind, TypeInfo};
use once_cell::sync::OnceCell;
use std::{
    convert::TryInto,
    ffi::{CStr, CString},
    fmt::{self, Formatter},
    os::raw::c_char,
    ptr::NonNull,
    str,
    sync::Once,
};

/// Represents a reference to a type declaration.
#[repr(C)]
#[derive(Clone, Eq)]
pub struct TypeRef {
    /// Type GUID
    pub guid: Guid,
    /// Type name
    pub name: *const c_char,
    /// Type data
    pub data: TypeRefData,
}

/// The kind of type reference and its corresponding data.
#[repr(u8)]
#[derive(Clone, Debug, Eq)]
pub enum TypeRefData {
    /// A primitve type (i.e. u8, i16, f32, etc.)
    Primitive,
    /// A struct type
    Struct {
        /// The rounded size of the type in bytes without any padding
        size_in_bits: u32,
        /// Struct memory kind
        memory_kind: StructMemoryKind,
    },
    /// A pointer type
    Ptr {
        /// Whether the pointer is mutable
        is_mut: bool,
        /// The type of the pointee
        pointee: NonNull<TypeRef>,
    },
    /// An unknown type
    Unknown,
}

impl TypeRef {
    /// Returns the type's name.
    pub fn name(&self) -> &str {
        unsafe { str::from_utf8_unchecked(CStr::from_ptr(self.name).to_bytes()) }
    }
}

impl fmt::Debug for TypeRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("TypeRefType")
            .field("guid", &self.guid)
            .field("name", &self.name())
            .field("data", &self.data)
            .finish()
    }
}

impl fmt::Display for TypeRef {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.data {
            TypeRefData::Primitive | TypeRefData::Struct { .. } | TypeRefData::Unknown => {
                write!(f, "{}", self.name())?
            }
            TypeRefData::Ptr { is_mut, pointee } => {
                write!(f, "*{} {}", if is_mut { "mut" } else { "const" }, unsafe {
                    pointee.as_ref().name()
                })?;
            }
        }

        Ok(())
    }
}

impl PartialEq for TypeRef {
    fn eq(&self, other: &Self) -> bool {
        self.guid == other.guid
    }
}

impl PartialEq for TypeRefData {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (TypeRefData::Primitive, TypeRefData::Primitive)
            | (TypeRefData::Unknown, TypeRefData::Unknown)
            // The `Guid` is enough to determine a struct's uniqueness
            | (TypeRefData::Struct { .. }, TypeRefData::Struct { .. }) => true,
            (
                TypeRefData::Ptr {
                    is_mut: lhs_is_mut,
                    pointee: lhs_pointee,
                },
                TypeRefData::Ptr {
                    is_mut: rhs_is_mut,
                    pointee: rhs_pointee,
                },
            ) => {
                lhs_is_mut == rhs_is_mut && unsafe { lhs_pointee.as_ref() == rhs_pointee.as_ref() }
            }
            _ => false,
        }
    }
}

impl TypeRefData {
    /// Returns the size in bytes and memory kind of the type, if it is a struct.
    pub fn as_struct_data(&self) -> Option<(usize, StructMemoryKind)> {
        match &self {
            TypeRefData::Struct {
                size_in_bits,
                memory_kind,
            } => {
                let size_in_bytes = ((size_in_bits + 7) / 8)
                    .try_into()
                    .expect("cannot covert size in bytes to platform size");

                Some((size_in_bytes, *memory_kind))
            }
            _ => None,
        }
    }
}

unsafe impl Send for TypeRef {}
unsafe impl Sync for TypeRef {}

/// A trait that defines that a type can be statically return a `TypeRef`.
pub trait HasStaticTypeRef: 'static {
    /// Returns a reference to the `TypeRef` for this type.
    fn type_ref() -> &'static TypeRef;
}

impl<T: HasStaticTypeInfo> HasStaticTypeRef for T {
    fn type_ref() -> &'static TypeRef {
        static mut VALUE: Option<TypeRef> = None;
        static INIT: Once = Once::new();

        unsafe {
            INIT.call_once(|| {
                let type_info = T::type_info();
                VALUE = Some(TypeRef {
                    guid: type_info.guid,
                    name: type_info.name,
                    data: TypeRefData::Primitive,
                });
            });
            VALUE.as_ref().unwrap()
        }
    }
}

/// Every type that has a `TypeRef` also has a valid constant pointer to that `TypeRef`.
impl<T: HasStaticTypeRef> HasStaticTypeRef for *const T {
    fn type_ref() -> &'static TypeRef {
        static mut VALUE: Option<TypeRef> = None;
        static INIT: Once = Once::new();

        unsafe {
            INIT.call_once(|| {
                static TYPE_REF_NAME: OnceCell<CString> = OnceCell::new();

                let type_ref_name = TYPE_REF_NAME.get_or_init(|| {
                    CString::new(format!("*const {}", T::type_ref().name())).unwrap()
                });

                VALUE = Some(TypeRef {
                    guid: Guid(md5::compute(&type_ref_name.as_bytes()).0),
                    name: type_ref_name.as_ptr(),
                    data: TypeRefData::Ptr {
                        is_mut: false,
                        pointee: T::type_ref().into(),
                    },
                });
            });
            VALUE.as_ref().unwrap()
        }
    }
}

/// Every type that has a `TypeRef` also has a valid mutable pointer to that `TypeRef`.
impl<T: HasStaticTypeRef> HasStaticTypeRef for *mut T {
    fn type_ref() -> &'static TypeRef {
        static mut VALUE: Option<TypeRef> = None;
        static INIT: Once = Once::new();

        unsafe {
            INIT.call_once(|| {
                static TYPE_REF_NAME: OnceCell<CString> = OnceCell::new();

                let type_ref_name = TYPE_REF_NAME.get_or_init(|| {
                    CString::new(format!("*const {}", T::type_ref().name())).unwrap()
                });

                VALUE = Some(TypeRef {
                    guid: Guid(md5::compute(&type_ref_name.as_bytes()).0),
                    name: type_ref_name.as_ptr(),
                    data: TypeRefData::Ptr {
                        is_mut: false,
                        pointee: T::type_ref().into(),
                    },
                });
            });
            VALUE.as_ref().unwrap()
        }
    }
}

macro_rules! impl_unknown_type_ref {
    ($(
        $ty:ty => $name:tt
    ),+) => {
        $(
            impl crate::type_ref::HasStaticTypeRef for $ty {
                fn type_ref() -> &'static TypeRef {
                    static TYPE_REF: once_cell::sync::OnceCell<TypeRef> = once_cell::sync::OnceCell::new();
                    TYPE_REF.get_or_init(|| {
                        static TYPE_REF_NAME: once_cell::sync::OnceCell<CString> = once_cell::sync::OnceCell::new();
                        let type_ref_name: &'static std::ffi::CString = TYPE_REF_NAME
                            .get_or_init(|| std::ffi::CString::new($name).unwrap());

                        TypeRef {
                            guid: Guid(md5::compute(&type_ref_name.as_bytes()).0),
                            name: type_ref_name.as_ptr(),
                            data: TypeRefData::Unknown,
                        }
                    })
                }
            }
        )+
    }
}

impl_unknown_type_ref!(
    std::ffi::c_void => "core::void",
    TypeInfo => "TypeInfo"
);

#[cfg(test)]
mod tests {
    use crate::{
        test_utils::{fake_type_ref, FAKE_TYPE_NAME},
        HasStaticTypeRef, TypeRefData,
    };
    use std::ffi::CString;

    #[test]
    fn test_type_ref_type_name() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_ref = fake_type_ref(&type_name, TypeRefData::Primitive);

        assert_eq!(format!("{}", type_ref), FAKE_TYPE_NAME);
    }

    #[test]
    fn test_type_ref_eq() {
        let i8_ty = i8::type_ref();
        assert_eq!(i8_ty, i8_ty);

        let u8_ty = u8::type_ref();
        assert_eq!(u8_ty, u8_ty);

        assert_ne!(i8_ty, u8_ty);

        let const_ptr_ty = <*const u8>::type_ref();
        assert_eq!(const_ptr_ty, const_ptr_ty);

        let mut_ptr_ty = <*mut u8>::type_ref();
        assert_eq!(mut_ptr_ty, mut_ptr_ty);

        assert_ne!(const_ptr_ty, mut_ptr_ty);

        let const_ptr_mut_ptr = <*const *mut u8>::type_ref();
        assert_eq!(const_ptr_mut_ptr, const_ptr_mut_ptr);

        assert_ne!(const_ptr_ty, const_ptr_mut_ptr);
        assert_ne!(mut_ptr_ty, const_ptr_mut_ptr);
    }

    #[test]
    fn test_type_name() {
        assert_eq!(format!("{}", u8::type_ref()), "core::u8",);
        assert_eq!(
            format!("{}", <*const std::ffi::c_void>::type_ref()),
            "*const core::void"
        );
        assert_eq!(
            format!("{}", <*const *mut std::ffi::c_void>::type_ref()),
            "*const *mut core::void"
        );
        assert_eq!(
            format!("{}", <*const *const std::ffi::c_void>::type_ref()),
            "*const *const core::void"
        );
    }
}

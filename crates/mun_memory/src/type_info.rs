use std::hash::{Hash, Hasher};
use std::sync::Once;
use std::{
    alloc::Layout,
    fmt::{self, Formatter},
    sync::Arc,
};

use itertools::izip;

use abi::static_type_map::StaticTypeMap;
use abi::{Guid, StructMemoryKind};

use crate::{type_table::TypeTable, TryFromAbiError, TypeFields};

/// A linked version of [`mun_abi::TypeInfo`] that has resolved all occurrences of `TypeId` with `TypeInfo`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeInfo {
    /// Type name
    pub name: String,
    // TODO: Move layout to TypeInfoData
    /// The memory layout of the type
    pub layout: Layout,
    /// Type group
    pub data: TypeInfoData,
}

/// A linked version of [`mun_abi::TypeInfoData`] that has resolved all occurrences of `TypeId` with `TypeInfo`.
#[repr(u8)]
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum TypeInfoData {
    /// Primitive types (i.e. `()`, `bool`, `float`, `int`, etc.)
    Primitive(Guid),
    /// Struct types (i.e. record, tuple, or unit structs)
    Struct(StructInfo),
    /// A pointer to another type
    Pointer(PointerInfo),
}

/// A linked version of [`mun_abi::StructInfo`] that has resolved all occurrences of `TypeId` with `TypeInfo`.  
#[derive(Clone, Debug)]
pub struct StructInfo {
    /// The unique identifier of this struct
    pub guid: Guid,
    /// Struct fields
    pub fields: Vec<FieldInfo>,
    /// Struct memory kind
    pub memory_kind: abi::StructMemoryKind,
}

/// A linked version of [`mun_abi::PointerInfo`] that has resolved all occurances of `TypeId` with `TypeInfo`.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct PointerInfo {
    /// The type to which is pointed
    pub pointee: Arc<TypeInfo>,
    /// Whether or not the pointer is mutable
    pub mutable: bool,
}

impl From<StructInfo> for TypeInfoData {
    fn from(s: StructInfo) -> Self {
        TypeInfoData::Struct(s)
    }
}

impl From<PointerInfo> for TypeInfoData {
    fn from(p: PointerInfo) -> Self {
        TypeInfoData::Pointer(p)
    }
}

impl fmt::Display for TypeInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl TypeFields for Arc<TypeInfo> {
    fn fields(&self) -> &[FieldInfo] {
        if let TypeInfoData::Struct(s) = &self.data {
            &s.fields
        } else {
            &[]
        }
    }
}

impl Hash for TypeInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.data.hash(state)
    }
}

impl Hash for StructInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.guid.hash(state)
    }
}

impl PartialEq for StructInfo {
    fn eq(&self, other: &Self) -> bool {
        self.guid == other.guid
    }
}
impl Eq for StructInfo {}

impl TypeInfo {
    /// Returns true if this instance represents the TypeInfo of the given type.
    ///
    /// ```rust
    /// # use mun_memory::HasStaticTypeInfo;
    /// assert!(i64::type_info().equals::<i64>());
    /// assert!(!i64::type_info().equals::<f64>())
    /// ```
    pub fn equals<T: HasStaticTypeInfo>(&self) -> bool {
        T::type_info().as_ref() == self
    }

    /// Returns whether this is a fundamental type.
    pub fn is_primitive(&self) -> bool {
        self.data.is_primitive()
    }

    /// Returns whether this is a struct type.
    pub fn is_struct(&self) -> bool {
        self.data.is_struct()
    }

    /// Returns whether this is a pointer type.
    pub fn is_pointer(&self) -> bool {
        self.data.is_pointer()
    }

    /// Returns true if this type is a concrete type. This is the case for any type that doesn't
    /// refer to another type like a pointer.
    pub fn is_concrete(&self) -> bool {
        match &self.data {
            TypeInfoData::Primitive(_) => true,
            TypeInfoData::Struct(_) => true,
            TypeInfoData::Pointer(_) => false
        }
    }

    /// Returns the GUID associated with this instance if this instance represents a concrete type.
    pub fn as_concrete(&self) -> Option<&Guid> {
        match &self.data {
            TypeInfoData::Primitive(g) => Some(g),
            TypeInfoData::Struct(s) => Some(&s.guid),
            TypeInfoData::Pointer(_) => None
        }
    }

    /// Retrieves the type's struct information, if available.
    pub fn as_struct(&self) -> Option<&StructInfo> {
        if let TypeInfoData::Struct(s) = &self.data {
            Some(s)
        } else {
            None
        }
    }

    /// Retrieves the type's pointer information, if available.
    pub fn as_pointer(&self) -> Option<&PointerInfo> {
        if let TypeInfoData::Pointer(p) = &self.data {
            Some(p)
        } else {
            None
        }
    }

    /// Returns whether the type is allocated on the stack.
    pub fn is_stack_allocated(&self) -> bool {
        match &self.data {
            TypeInfoData::Primitive(_) => true,
            TypeInfoData::Struct(s) => s.memory_kind == StructMemoryKind::Value,
            TypeInfoData::Pointer(_) => true,
        }
    }

    /// Tries to convert from an `abi::TypeInfo`.
    pub fn try_from_abi(
        type_info: &abi::TypeInfo,
        type_table: &TypeTable,
    ) -> Result<TypeInfo, TryFromAbiError> {
        TypeInfoData::try_from_abi(&type_info.data, type_table).map(|data| TypeInfo {
            name: type_info.name().to_owned(),
            layout: Layout::from_size_align(type_info.size_in_bytes(), type_info.alignment())
                .unwrap_or_else(|_| {
                    panic!(
                        "TypeInfo contains invalid size and alignment (size: {}, align: {})",
                        type_info.size_in_bytes(),
                        type_info.alignment()
                    )
                }),
            data,
        })
    }
}

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

    /// Tries to convert from an `abi::TypeInfoData`.
    pub fn try_from_abi(
        type_info_data: &abi::TypeInfoData,
        type_table: &TypeTable,
    ) -> Result<TypeInfoData, TryFromAbiError> {
        match type_info_data {
            abi::TypeInfoData::Primitive(guid) => Ok(TypeInfoData::Primitive(guid.clone())),
            abi::TypeInfoData::Struct(s) => {
                StructInfo::try_from_abi(s, type_table).map(TypeInfoData::Struct)
            }
            abi::TypeInfoData::Pointer(p) => {
                PointerInfo::try_from_abi(p, type_table).map(TypeInfoData::Pointer)
            }
        }
    }
}

impl StructInfo {
    /// Returns the `TypeInfo` and offset corresponding to the field matching the specified `field_name`, if it exists.
    pub fn find_field_by_name<S: AsRef<str>>(&self, field_name: S) -> Option<&FieldInfo> {
        self.fields
            .iter()
            .find(|field| field.name == field_name.as_ref())
    }

    /// Tries to convert from an `abi::StructInfo`.
    pub fn try_from_abi(
        struct_info: &abi::StructInfo,
        type_table: &TypeTable,
    ) -> Result<StructInfo, TryFromAbiError> {
        let fields: Result<Vec<FieldInfo>, TryFromAbiError> = izip!(
            struct_info.field_names(),
            struct_info.field_types(),
            struct_info.field_offsets()
        )
        .map(|(name, type_id, offset)| {
            type_table
                .find_type_info_by_id(type_id)
                .ok_or_else(|| TryFromAbiError::UnknownTypeId(type_id.clone()))
                .map(|type_info| FieldInfo {
                    name: name.to_owned(),
                    type_info,
                    offset: *offset,
                })
        })
        .collect();

        fields.map(|fields| StructInfo {
            guid: struct_info.guid.clone(),
            fields,
            memory_kind: struct_info.memory_kind,
        })
    }
}

impl PointerInfo {
    /// Tries to convert from an `abi::PointerInfo`.
    pub fn try_from_abi(
        pointer_info: &abi::PointerInfo,
        type_table: &TypeTable,
    ) -> Result<PointerInfo, TryFromAbiError> {
        Ok(PointerInfo {
            pointee: type_table
                .find_type_info_by_id(&pointer_info.pointee)
                .ok_or_else(|| TryFromAbiError::UnknownTypeId(pointer_info.pointee.clone()))?,
            mutable: pointer_info.mutable,
        })
    }
}

/// A linked version of a struct field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldInfo {
    /// The field's name
    pub name: String,
    /// The field's type
    pub type_info: Arc<TypeInfo>,
    /// The field's offset
    pub offset: u16,
    // TODO: Field accessibility levels
    // const MunPrivacy_t *field_privacies,
}

/// A trait that defines static type information for types that can provide it.
pub trait HasStaticTypeInfo {
    fn type_info() -> &'static Arc<TypeInfo>;
}

macro_rules! impl_primitive_type {
    ($($ty:ty),+) => {
        $(
            impl HasStaticTypeInfo for $ty {
                fn type_info() -> &'static Arc<TypeInfo> {
                    static TYPE_INFO: once_cell::sync::OnceCell<Arc<TypeInfo>> = once_cell::sync::OnceCell::new();
                    TYPE_INFO.get_or_init(|| {
                        let type_info = <$ty as abi::HasStaticTypeInfo>::type_info();
                        match type_info.data {
                            abi::TypeInfoData::Primitive(guid) => {
                                Arc::new(TypeInfo {
                                    name: type_info.name().to_owned(),
                                    layout:  Layout::from_size_align(type_info.size_in_bytes(), type_info.alignment())
                                        .expect("TypeInfo contains invalid size and alignment."),
                                    data: TypeInfoData::Primitive(guid.clone())
                                })
                            }
                            _ => unreachable!("invalid primitive type")
                        }
                    })
                }
            }
        )+
    }
}

impl_primitive_type!(
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    f32,
    f64,
    bool,
    (),
    std::ffi::c_void
);

/// Every type that has at least a type name also has a valid pointer type name
impl<T: HasStaticTypeInfo + 'static> HasStaticTypeInfo for *mut T {
    fn type_info() -> &'static Arc<TypeInfo> {
        static mut VALUE: Option<StaticTypeMap<Arc<TypeInfo>>> = None;
        static INIT: Once = Once::new();

        let map = unsafe {
            INIT.call_once(|| {
                VALUE = Some(StaticTypeMap::default());
            });
            VALUE.as_ref().unwrap()
        };

        map.call_once::<T, _>(|| {
            let element_type = T::type_info();
            let name = format!("*mut {}", &element_type.name);
            let size_in_bits = std::mem::size_of::<*mut T>();
            let alignment = std::mem::align_of::<*mut T>();
            Arc::new(TypeInfo {
                name,
                layout: Layout::from_size_align(size_in_bits, alignment)
                    .expect("invalid layout for static type"),
                data: PointerInfo {
                    pointee: element_type.clone(),
                    mutable: true,
                }
                .into(),
            })
        })
    }
}

/// Every type that has at least a type name also has a valid pointer type name
impl<T: HasStaticTypeInfo + 'static> HasStaticTypeInfo for *const T {
    fn type_info() -> &'static Arc<TypeInfo> {
        static mut VALUE: Option<StaticTypeMap<Arc<TypeInfo>>> = None;
        static INIT: Once = Once::new();

        let map = unsafe {
            INIT.call_once(|| {
                VALUE = Some(StaticTypeMap::default());
            });
            VALUE.as_ref().unwrap()
        };

        map.call_once::<T, _>(|| {
            let element_type = T::type_info();
            let name = format!("*const {}", &element_type.name);
            let size_in_bits = std::mem::size_of::<*mut T>();
            let alignment = std::mem::align_of::<*mut T>();
            Arc::new(TypeInfo {
                name,
                layout: Layout::from_size_align(size_in_bits, alignment)
                    .expect("invalid layout for static type"),
                data: PointerInfo {
                    pointee: element_type.clone(),
                    mutable: false,
                }
                .into(),
            })
        })
    }
}

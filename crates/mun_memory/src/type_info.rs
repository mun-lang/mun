use crate::{type_table::TypeTable, TypeFields};
use abi::StructMemoryKind;
use itertools::izip;
use std::{
    alloc::Layout,
    fmt::{self, Formatter},
    sync::Arc,
};

/// A linked version of [`mun_abi::TypeInfo`] that has resolved all occurrences of `TypeId` with `TypeInfo`.
#[derive(Clone, Debug)]
pub struct TypeInfo {
    /// Type ID
    pub id: abi::TypeId,
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
#[derive(Clone, Debug)]
pub enum TypeInfoData {
    /// Primitive types (i.e. `()`, `bool`, `float`, `int`, etc.)
    Primitive,
    /// Struct types (i.e. record, tuple, or unit structs)
    Struct(StructInfo),
}

/// A linked version of [`mun_abi::StructInfo`] that has resolved all occurrences of `TypeId` with `TypeInfo`.  
#[derive(Clone, Debug)]
pub struct StructInfo {
    /// Struct fields
    pub fields: Vec<FieldInfo>,
    /// Struct memory kind
    pub memory_kind: abi::StructMemoryKind,
}

impl fmt::Display for TypeInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl PartialEq for TypeInfo {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for TypeInfo {}

impl std::hash::Hash for TypeInfo {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
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

impl TypeInfo {
    /// Returns whether this is a fundamental type.
    pub fn is_primitive(&self) -> bool {
        self.data.is_primitive()
    }

    /// Returns whether this is a struct type.
    pub fn is_struct(&self) -> bool {
        self.data.is_struct()
    }

    /// Retrieves the type's struct information, if available.
    pub fn as_struct(&self) -> Option<&StructInfo> {
        if let TypeInfoData::Struct(s) = &self.data {
            Some(s)
        } else {
            None
        }
    }

    /// Returns whether the type is allocated on the stack.
    pub fn is_stack_allocated(&self) -> bool {
        match &self.data {
            TypeInfoData::Primitive => true,
            TypeInfoData::Struct(s) => s.memory_kind == StructMemoryKind::Value,
        }
    }

    pub fn try_from_abi(type_info: &abi::TypeInfo, type_table: &TypeTable) -> Option<TypeInfo> {
        TypeInfoData::try_from_abi(&type_info.data, type_table).map(|data| TypeInfo {
            id: type_info.id.clone(),
            name: type_info.name().to_owned(),
            layout: Layout::from_size_align(type_info.size_in_bytes(), type_info.alignment())
                .expect("TypeInfo contains invalid size and alignment."),
            data,
        })
    }
}

impl TypeInfoData {
    /// Returns whether this is a fundamental type.
    pub fn is_primitive(&self) -> bool {
        matches!(self, TypeInfoData::Primitive)
    }

    /// Returns whether this is a struct type.
    pub fn is_struct(&self) -> bool {
        matches!(self, TypeInfoData::Struct(_))
    }

    pub fn try_from_abi(
        type_info_data: &abi::TypeInfoData,
        type_table: &TypeTable,
    ) -> Option<TypeInfoData> {
        match type_info_data {
            abi::TypeInfoData::Primitive => Some(TypeInfoData::Primitive),
            abi::TypeInfoData::Struct(s) => {
                StructInfo::try_from_abi(s, type_table).map(TypeInfoData::Struct)
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

    pub fn try_from_abi(
        struct_info: &abi::StructInfo,
        type_table: &TypeTable,
    ) -> Option<StructInfo> {
        let fields: Option<Vec<FieldInfo>> = izip!(
            struct_info.field_names(),
            struct_info.field_types(),
            struct_info.field_offsets()
        )
        .map(|(name, type_id, offset)| {
            type_table
                .find_type_info_by_id(type_id)
                .map(|type_info| FieldInfo {
                    name: name.to_owned(),
                    type_info,
                    offset: *offset,
                })
        })
        .collect();

        fields.map(|fields| StructInfo {
            fields,
            memory_kind: struct_info.memory_kind,
        })
    }
}

/// A linked version of a struct field.
#[derive(Clone, Debug)]
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
pub trait HasStaticTypeInfo: abi::HasStaticTypeInfo {
    fn type_info() -> Arc<TypeInfo>;
}

macro_rules! impl_primitive_type {
    ($($ty:ty),+) => {
        $(
            impl HasStaticTypeInfo for $ty {
                fn type_info() -> Arc<TypeInfo> {
                    static TYPE_INFO: once_cell::sync::OnceCell<Arc<TypeInfo>> = once_cell::sync::OnceCell::new();
                    TYPE_INFO.get_or_init(|| {
                        let type_info = <$ty as abi::HasStaticTypeInfo>::type_info();
                        Arc::new(TypeInfo {
                            id: type_info.id.clone(),
                            name: type_info.name().to_owned(),
                            layout:  Layout::from_size_align(type_info.size_in_bytes(), type_info.alignment())
                                .expect("TypeInfo contains invalid size and alignment."),
                            data: TypeInfoData::Primitive
                        })
                    }).clone()
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
    ()
);

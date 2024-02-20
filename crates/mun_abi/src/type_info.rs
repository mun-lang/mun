use std::{
    convert::TryInto,
    ffi::CStr,
    fmt::{self, Debug, Formatter},
    os::raw::c_char,
    str,
};

use crate::{type_id::TypeId, Guid, StructDefinition};

/// Represents the type declaration for a type that is exported by an assembly.
///
/// When multiple Mun modules reference the same type, only one module exports
/// the type; the module that contains the type definition. All the other Mun
/// modules reference the type through a [`TypeId`].
///
/// The modules that defines the type exports the data to reduce the filesize of
/// the assemblies and to ensure only one definition exists. When linking all
/// assemblies together the type definitions from all assemblies are loaded and
/// the information is shared to modules that reference the type.
///
/// TODO: add support for polymorphism, enumerations, type parameters, generic
/// type definitions, and   constructed generic types.
#[repr(C)]
pub struct TypeDefinition<'a> {
    /// Type name
    pub name: *const c_char,
    /// The exact size of the type in bits without any padding
    pub(crate) size_in_bits: u32,
    /// The alignment of the type
    pub(crate) alignment: u8,
    /// Type group
    pub data: TypeDefinitionData<'a>,
}

impl<'a> Debug for TypeDefinition<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("TypeDefinition")
            .field("name", &self.name())
            .field("size_in_bits", &self.size_in_bits)
            .field("alignment", &self.alignment)
            .field("data", &self.data)
            .finish()
    }
}

#[cfg(feature = "serde")]
impl<'a> serde::Serialize for TypeDefinition<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut s = serializer.serialize_struct("TypeDefinition", 4)?;
        s.serialize_field("name", self.name())?;
        s.serialize_field("size_in_bits", &self.size_in_bits)?;
        s.serialize_field("alignment", &self.alignment)?;
        s.serialize_field("data", &self.data)?;
        s.end()
    }
}

/// Contains data specific to a group of types that illicit the same
/// characteristics.
#[repr(u8)]
#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum TypeDefinitionData<'a> {
    /// Struct types (i.e. record, tuple, or unit structs)
    Struct(StructDefinition<'a>),
}

impl<'a> TypeDefinition<'a> {
    /// Returns true if this instance is an instance of the given `TypeId`.
    pub fn is_instance_of(&self, type_id: &TypeId<'a>) -> bool {
        match (&self.data, type_id) {
            (TypeDefinitionData::Struct(s), TypeId::Concrete(guid)) => &s.guid == guid,
            _ => false,
        }
    }

    /// Returns the type's name.
    pub fn name(&self) -> &str {
        unsafe { str::from_utf8_unchecked(CStr::from_ptr(self.name).to_bytes()) }
    }

    /// Returns the GUID if this type represents a concrete type.
    pub fn as_concrete(&self) -> &Guid {
        match &self.data {
            TypeDefinitionData::Struct(s) => &s.guid,
        }
    }

    /// Retrieves the type's struct information, if available.
    pub fn as_struct(&self) -> Option<&StructDefinition<'_>> {
        let TypeDefinitionData::Struct(s) = &self.data;
        Some(s)
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
        self.alignment.into()
    }
}

impl<'a> fmt::Display for TypeDefinition<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl<'a> PartialEq for TypeDefinition<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.size_in_bits == other.size_in_bits
            && self.alignment == other.alignment
            && self.data == other.data
    }
}

impl<'a> Eq for TypeDefinition<'a> {}

unsafe impl<'a> Send for TypeDefinition<'a> {}
unsafe impl<'a> Sync for TypeDefinition<'a> {}

impl<'a> TypeDefinitionData<'a> {
    /// Returns whether this is a struct type.
    pub fn is_struct(&self) -> bool {
        matches!(self, TypeDefinitionData::Struct(_))
    }
}

/// A trait that defines that for a type we can statically return a type name.
pub trait HasStaticTypeName {
    /// Returns a reference to the [`TypeInfo`] for the type
    fn type_name() -> &'static CStr;
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use super::TypeDefinitionData;
    use crate::{
        test_utils::{fake_struct_definition, fake_type_definition, FAKE_TYPE_NAME},
        StructMemoryKind,
    };

    #[test]
    fn test_type_definition_name() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let field_names = &[];
        let field_types = &[];
        let field_offsets = &[];
        let struct_info = fake_struct_definition(
            &type_name,
            field_names,
            field_types,
            field_offsets,
            StructMemoryKind::default(),
        );

        let type_definition =
            fake_type_definition(&type_name, 1, 1, TypeDefinitionData::Struct(struct_info));
        assert_eq!(type_definition.name(), FAKE_TYPE_NAME);
    }

    #[test]
    fn test_type_definition_size_alignment() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let field_names = &[];
        let field_types = &[];
        let field_offsets = &[];
        let struct_info = fake_struct_definition(
            &type_name,
            field_names,
            field_types,
            field_offsets,
            StructMemoryKind::default(),
        );

        let type_definition =
            fake_type_definition(&type_name, 24, 8, TypeDefinitionData::Struct(struct_info));

        assert_eq!(type_definition.size_in_bits(), 24);
        assert_eq!(type_definition.size_in_bytes(), 3);
        assert_eq!(type_definition.alignment(), 8);
    }

    #[test]
    fn test_type_definition_group_struct() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let field_names = &[];
        let field_types = &[];
        let field_offsets = &[];
        let struct_info = fake_struct_definition(
            &type_name,
            field_names,
            field_types,
            field_offsets,
            StructMemoryKind::default(),
        );

        let type_definition =
            fake_type_definition(&type_name, 1, 1, TypeDefinitionData::Struct(struct_info));
        assert!(type_definition.data.is_struct());
    }

    #[test]
    fn test_type_definition_eq() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let field_names = &[];
        let field_types = &[];
        let field_offsets = &[];
        let struct_info = fake_struct_definition(
            &type_name,
            field_names,
            field_types,
            field_offsets,
            StructMemoryKind::default(),
        );

        let type_definition =
            fake_type_definition(&type_name, 1, 1, TypeDefinitionData::Struct(struct_info));
        assert_eq!(type_definition, type_definition);
    }
}

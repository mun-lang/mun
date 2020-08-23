use crate::TypeInfo;
use std::{ffi::CStr, os::raw::c_char, slice, str};

/// Represents a struct declaration.
#[repr(C)]
pub struct StructInfo {
    /// Struct fields' names
    pub field_names: *const *const c_char,
    /// Struct fields' information
    pub(crate) field_types: *const *const TypeInfo,
    /// Struct fields' offsets
    pub(crate) field_offsets: *const u16,
    // TODO: Field accessibility levels
    // const MunPrivacy_t *field_privacies,
    /// Number of fields
    pub(crate) num_fields: u16,
    // TODO: Add struct accessibility level
    /// Struct memory kind
    pub memory_kind: StructMemoryKind,
}

/// Represents the kind of memory management a struct uses.
#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructMemoryKind {
    /// A garbage collected struct is allocated on the heap and uses reference semantics when passed
    /// around.
    GC,

    /// A value struct is allocated on the stack and uses value semantics when passed around.
    ///
    /// NOTE: When a value struct is used in an external API, a wrapper is created that _pins_ the
    /// value on the heap. The heap-allocated value needs to be *manually deallocated*!
    Value,
}

impl StructInfo {
    /// Returns the struct's field names.
    pub fn field_names(&self) -> impl Iterator<Item = &str> {
        let field_names = if self.num_fields == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(self.field_names, self.num_fields as usize) }
        };

        field_names
            .iter()
            .map(|n| unsafe { str::from_utf8_unchecked(CStr::from_ptr(*n).to_bytes()) })
    }

    /// Returns the struct's field types.
    pub fn field_types(&self) -> &[&TypeInfo] {
        if self.num_fields == 0 {
            &[]
        } else {
            unsafe {
                slice::from_raw_parts(
                    self.field_types.cast::<&TypeInfo>(),
                    self.num_fields as usize,
                )
            }
        }
    }

    /// Returns the struct's field offsets.
    pub fn field_offsets(&self) -> &[u16] {
        if self.num_fields == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(self.field_offsets, self.num_fields as usize) }
        }
    }

    /// Returns the number of struct fields.
    pub fn num_fields(&self) -> usize {
        self.num_fields.into()
    }

    /// Returns the index of the field matching the specified `field_name`.
    pub fn find_field_index(
        type_name: &str,
        struct_info: &StructInfo,
        field_name: &str,
    ) -> Result<usize, String> {
        struct_info
            .field_names()
            .enumerate()
            .find(|(_, name)| *name == field_name)
            .map(|(idx, _)| idx)
            .ok_or_else(|| {
                format!(
                    "Struct `{}` does not contain field `{}`.",
                    type_name, field_name
                )
            })
    }
}

impl Default for StructMemoryKind {
    fn default() -> Self {
        StructMemoryKind::GC
    }
}

impl From<StructMemoryKind> for u64 {
    fn from(kind: StructMemoryKind) -> Self {
        match kind {
            StructMemoryKind::GC => 0,
            StructMemoryKind::Value => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StructMemoryKind;
    use crate::{
        test_utils::{fake_struct_info, fake_type_info, FAKE_FIELD_NAME, FAKE_TYPE_NAME},
        TypeGroup,
    };
    use std::ffi::CString;

    #[test]
    fn test_struct_info_fields_none() {
        let field_names = &[];
        let field_types = &[];
        let field_offsets = &[];
        let struct_info =
            fake_struct_info(field_names, field_types, field_offsets, Default::default());

        assert_eq!(struct_info.field_names().count(), 0);
        assert_eq!(struct_info.field_types(), field_types);
        assert_eq!(struct_info.field_offsets(), field_offsets);
    }

    #[test]
    fn test_struct_info_fields_some() {
        let field_name = CString::new(FAKE_FIELD_NAME).expect("Invalid fake field name.");
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name, TypeGroup::FundamentalTypes, 1, 1);

        let field_names = &[field_name.as_ptr()];
        let field_types = &[&type_info];
        let field_offsets = &[1];
        let struct_info =
            fake_struct_info(field_names, field_types, field_offsets, Default::default());

        for (lhs, rhs) in struct_info.field_names().zip([FAKE_FIELD_NAME].iter()) {
            assert_eq!(lhs, *rhs)
        }
        assert_eq!(struct_info.field_types(), field_types);
        assert_eq!(struct_info.field_offsets(), field_offsets);
    }

    #[test]
    fn test_struct_info_memory_kind_gc() {
        let struct_memory_kind = StructMemoryKind::GC;
        let struct_info = fake_struct_info(&[], &[], &[], struct_memory_kind.clone());

        assert_eq!(struct_info.memory_kind, struct_memory_kind);
    }

    #[test]
    fn test_struct_info_memory_kind_value() {
        let struct_memory_kind = StructMemoryKind::Value;
        let struct_info = fake_struct_info(&[], &[], &[], struct_memory_kind.clone());

        assert_eq!(struct_info.memory_kind, struct_memory_kind);
    }
}

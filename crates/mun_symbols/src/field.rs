use crate::prelude::*;

/// Reflection information about a type field.
#[derive(Debug)]
pub struct FieldInfo {
    pub name: String,
    pub privacy: Privacy,
    pub type_info: &'static TypeInfo,
}

impl FieldInfo {
    /// Constructs a new `TypeInfo`.
    pub fn new(name: &str, privacy: Privacy, type_info: &'static TypeInfo) -> FieldInfo {
        Self {
            name: name.to_string(),
            privacy,
            type_info,
        }
    }
}

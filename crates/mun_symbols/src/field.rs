use crate::prelude::*;

/// Reflection information about a type field.
#[derive(Debug)]
pub struct FieldInfo {
    name: String,
    privacy: Privacy,
    type_info: &'static TypeInfo,
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

    /// Retrieves the field's type information.
    pub fn type_info(&self) -> &TypeInfo {
        self.type_info
    }
}

impl MemberInfo for FieldInfo {
    fn name(&self) -> &str {
        &self.name
    }

    fn privacy(&self) -> Privacy {
        self.privacy
    }

    fn is_private(&self) -> bool {
        self.privacy == Privacy::Private
    }

    fn is_public(&self) -> bool {
        self.privacy == Privacy::Public
    }
}

use crate::TypeId;

/// Represents an array declaration.
#[repr(C)]
#[derive(Debug)]
pub struct ArrayInfo {
    pub(crate) element_type: TypeId,
}

impl ArrayInfo {
    /// Returns the array's element type
    pub fn element_type(&self) -> &TypeId {
        &self.element_type
    }
}

#[cfg(test)]
mod tests {
    use crate::test_utils::fake_array_info;
    use crate::{
        test_utils::{fake_type_info, FAKE_TYPE_NAME},
        TypeInfoData,
    };
    use std::ffi::CString;

    #[test]
    fn test_array_info() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name, 1, 1, TypeInfoData::Primitive);
        let array_info = fake_array_info(&type_info);

        assert_eq!(array_info.element_type().guid, type_info.guid);
    }
}

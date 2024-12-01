use std::{ffi::CStr, os::raw::c_char, slice, str};

use crate::{FunctionDefinition, TypeDefinition};

/// Represents a module declaration.
#[repr(C)]
pub struct ModuleInfo<'a> {
    /// Module path
    pub(crate) path: *const c_char,
    /// Module functions
    pub(crate) functions: *const FunctionDefinition<'a>,
    /// Module types
    pub(crate) types: *const TypeDefinition<'a>,
    /// Number of module functions
    pub num_functions: u32,
    /// Number of module types
    pub num_types: u32,
}

impl<'a> ModuleInfo<'a> {
    /// Returns the module's full path.
    pub fn path(&self) -> &str {
        unsafe { str::from_utf8_unchecked(CStr::from_ptr(self.path).to_bytes()) }
    }

    // /// Finds the type's fields that match `filter`.
    // pub fn find_fields(&self, filter: fn(&&FieldInfo) -> bool) -> impl
    // Iterator<Item = &FieldInfo> {     self.fields.iter().map(|f|
    // *f).filter(filter) }

    // /// Retrieves the type's field with the specified `name`, if it exists.
    // pub fn get_field(&self, name: &str) -> Option<&FieldInfo> {
    //     self.fields.iter().find(|f| f.name == name).map(|f| *f)
    // }

    // /// Retrieves the type's fields.
    // pub fn get_fields(&self) -> impl Iterator<Item = &FieldInfo> {
    //     self.fields.iter().map(|f| *f)
    // }

    /// Returns the module's functions.
    pub fn functions(&self) -> &[FunctionDefinition<'a>] {
        if self.num_functions == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(self.functions, self.num_functions as usize) }
        }
    }

    /// Returns the module's types.
    pub fn types(&self) -> &[TypeDefinition<'a>] {
        if self.num_types == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(self.types, self.num_types as usize) }
        }
    }
}

unsafe impl Send for ModuleInfo<'_> {}
unsafe impl Sync for ModuleInfo<'_> {}

#[cfg(feature = "serde")]
impl serde::Serialize for ModuleInfo<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut s = serializer.serialize_struct("ModuleInfo", 3)?;
        s.serialize_field("path", self.path())?;
        s.serialize_field("functions", self.functions())?;
        s.serialize_field("types", self.types())?;
        s.end()
    }
}

#[cfg(test)]
mod tests {
    use std::{ffi::CString, ptr};

    use crate::{
        test_utils::{
            fake_fn_prototype, fake_module_info, fake_struct_definition, fake_type_definition,
            FAKE_FN_NAME, FAKE_MODULE_PATH, FAKE_STRUCT_NAME,
        },
        type_id::HasStaticTypeId,
        FunctionDefinition, StructMemoryKind, TypeDefinition, TypeDefinitionData,
    };

    #[test]
    fn test_module_info_path() {
        let module_path = CString::new(FAKE_MODULE_PATH).expect("Invalid fake module path.");
        let module = fake_module_info(&module_path, &[], &[]);

        assert_eq!(module.path(), FAKE_MODULE_PATH);
    }

    #[test]
    fn test_module_info_types_none() {
        let functions = &[];
        let types = &[];
        let module_path = CString::new(FAKE_MODULE_PATH).expect("Invalid fake module path.");
        let module = fake_module_info(&module_path, functions, types);

        assert_eq!(module.functions().len(), functions.len());
        assert_eq!(module.types().len(), types.len());
    }

    #[test]
    fn test_module_info_types_some() {
        let type_id = i32::type_id();
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_prototype = fake_fn_prototype(&fn_name, &[], Some(type_id.clone()));

        let fn_info = FunctionDefinition {
            prototype: fn_prototype,
            fn_ptr: ptr::null(),
        };
        let functions = &[fn_info];

        let struct_name = CString::new(FAKE_STRUCT_NAME).expect("Invalid fake struct name");
        let struct_info =
            fake_struct_definition(&struct_name, &[], &[], &[], StructMemoryKind::default());
        let type_info =
            fake_type_definition(&struct_name, 1, 1, TypeDefinitionData::Struct(struct_info));
        let types = [type_info];

        let module_path = CString::new(FAKE_MODULE_PATH).expect("Invalid fake module path.");
        let module = fake_module_info(&module_path, functions, &types);

        let result_functions = module.functions();
        assert_eq!(result_functions.len(), functions.len());
        for (lhs, rhs) in result_functions.iter().zip(functions.iter()) {
            assert_eq!(lhs.fn_ptr, rhs.fn_ptr);
            assert_eq!(lhs.prototype.name(), rhs.prototype.name());
            assert_eq!(
                lhs.prototype.signature.arg_types(),
                rhs.prototype.signature.arg_types()
            );
            assert_eq!(
                lhs.prototype.signature.return_type(),
                rhs.prototype.signature.return_type()
            );
        }

        let result_types: &[TypeDefinition<'_>] = module.types();
        assert_eq!(result_types.len(), types.len());
        for (lhs, rhs) in result_types.iter().zip(types.iter()) {
            assert_eq!(lhs, rhs);
            assert_eq!(lhs.name(), rhs.name());
            assert_eq!(lhs.data.is_struct(), rhs.data.is_struct());
            let TypeDefinitionData::Struct(lhs) = &lhs.data;
            let TypeDefinitionData::Struct(rhs) = &rhs.data;
            assert_eq!(lhs.field_types(), rhs.field_types());
        }
    }
}

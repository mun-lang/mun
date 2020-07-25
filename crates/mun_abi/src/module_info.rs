use crate::{FunctionDefinition, TypeInfo};
use std::{ffi::CStr, os::raw::c_char, slice, str};

/// Represents a module declaration.
#[repr(C)]
pub struct ModuleInfo {
    /// Module path
    pub(crate) path: *const c_char,
    /// Module functions
    pub(crate) functions: *const FunctionDefinition,
    /// Module types
    pub(crate) types: *const *const TypeInfo,
    /// Number of module functions
    pub num_functions: u32,
    /// Number of module types
    pub num_types: u32,
}

impl ModuleInfo {
    /// Returns the module's full path.
    pub fn path(&self) -> &str {
        unsafe { str::from_utf8_unchecked(CStr::from_ptr(self.path).to_bytes()) }
    }

    // /// Finds the type's fields that match `filter`.
    // pub fn find_fields(&self, filter: fn(&&FieldInfo) -> bool) -> impl Iterator<Item = &FieldInfo> {
    //     self.fields.iter().map(|f| *f).filter(filter)
    // }

    // /// Retrieves the type's field with the specified `name`, if it exists.
    // pub fn get_field(&self, name: &str) -> Option<&FieldInfo> {
    //     self.fields.iter().find(|f| f.name == name).map(|f| *f)
    // }

    // /// Retrieves the type's fields.
    // pub fn get_fields(&self) -> impl Iterator<Item = &FieldInfo> {
    //     self.fields.iter().map(|f| *f)
    // }

    /// Returns the module's functions.
    pub fn functions(&self) -> &[FunctionDefinition] {
        if self.num_functions == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(self.functions, self.num_functions as usize) }
        }
    }

    /// Returns the module's types.
    pub fn types(&self) -> &[&TypeInfo] {
        if self.num_types == 0 {
            &[]
        } else {
            unsafe {
                slice::from_raw_parts(self.types.cast::<&TypeInfo>(), self.num_types as usize)
            }
        }
    }
}

unsafe impl Send for ModuleInfo {}
unsafe impl Sync for ModuleInfo {}

#[cfg(test)]
mod tests {
    use crate::{
        test_utils::{
            fake_fn_prototype, fake_module_info, fake_struct_info, fake_struct_type_info,
            fake_type_info, FAKE_FN_NAME, FAKE_MODULE_PATH, FAKE_STRUCT_NAME, FAKE_TYPE_NAME,
        },
        FunctionDefinition, TypeGroup, TypeInfo,
    };
    use std::{ffi::CString, mem, ptr};

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
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name, TypeGroup::FundamentalTypes, 1, 1);

        let return_type = Some(&type_info);
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_prototype = fake_fn_prototype(&fn_name, &[], return_type);

        let fn_info = FunctionDefinition {
            prototype: fn_prototype,
            fn_ptr: ptr::null(),
        };
        let functions = &[fn_info];

        let struct_name = CString::new(FAKE_STRUCT_NAME).expect("Invalid fake struct name");
        let struct_info = fake_struct_info(&[], &[], &[], Default::default());
        let struct_type_info = fake_struct_type_info(&struct_name, struct_info, 1, 1);
        let types = &[unsafe { mem::transmute(&struct_type_info) }];

        let module_path = CString::new(FAKE_MODULE_PATH).expect("Invalid fake module path.");
        let module = fake_module_info(&module_path, functions, types);

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

        let result_types: &[&TypeInfo] = module.types();
        assert_eq!(result_types.len(), types.len());
        for (lhs, rhs) in result_types.iter().zip(types.iter()) {
            assert_eq!(lhs, rhs);
            assert_eq!(lhs.name(), rhs.name());
            assert_eq!(lhs.group, rhs.group);
            if lhs.group == TypeGroup::StructTypes {
                let lhs_struct = lhs.as_struct().unwrap();
                let rhs_struct = rhs.as_struct().unwrap();
                assert_eq!(lhs_struct.field_types(), rhs_struct.field_types());
            }
        }
    }
}

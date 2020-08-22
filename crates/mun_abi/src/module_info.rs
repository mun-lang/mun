use crate::FunctionDefinition;
use std::{ffi::CStr, os::raw::c_char, slice, str};

/// Represents a module declaration.
#[repr(C)]
pub struct ModuleInfo {
    /// Module path
    pub(crate) path: *const c_char,
    /// Module functions
    pub(crate) functions: *const FunctionDefinition,
    /// Number of module functions
    pub num_functions: u32,
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
}

unsafe impl Send for ModuleInfo {}
unsafe impl Sync for ModuleInfo {}

#[cfg(test)]
mod tests {
    use crate::test_utils::{fake_module_info, FAKE_MODULE_PATH};
    use std::ffi::CString;

    #[test]
    fn test_module_info_path() {
        let module_path = CString::new(FAKE_MODULE_PATH).expect("Invalid fake module path.");
        let module = fake_module_info(&module_path, &[]);

        assert_eq!(module.path(), FAKE_MODULE_PATH);
    }
}

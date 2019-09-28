use crate::prelude::*;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::ffi::{c_void, CStr};
use std::slice;

impl TypeInfo {
    pub fn name(&self) -> &str {
        unsafe { CStr::from_ptr(self.name) }
            .to_str()
            .expect("Type name contains invalid UTF8")
    }
}

impl PartialEq for TypeInfo {
    fn eq(&self, other: &Self) -> bool {
        self.guid == other.guid
    }
}

impl FunctionInfo {
    pub fn name(&self) -> &str {
        unsafe { CStr::from_ptr(self.name) }
            .to_str()
            .expect("Function name contains invalid UTF8")
    }

    pub fn privacy(&self) -> Privacy {
        self.privacy
    }

    pub fn arg_types(&self) -> &[TypeInfo] {
        if self.num_arg_types == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(self.arg_types, self.num_arg_types as usize) }
        }
    }

    pub fn return_type(&self) -> Option<&TypeInfo> {
        unsafe { self.return_type.as_ref() }
    }

    pub unsafe fn fn_ptr(&self) -> *const c_void {
        self.fn_ptr
    }
}

impl ModuleInfo {
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

    /// Finds the module's functions that match `filter`.
    pub fn find_functions<F>(&self, filter: F) -> impl Iterator<Item = &FunctionInfo>
    where
        F: FnMut(&&FunctionInfo) -> bool,
    {
        self.functions().iter().filter(filter)
    }

    /// Retrieves the module's function with the specified `name`, if it exists.
    pub fn get_function(&self, name: &str) -> Option<&FunctionInfo> {
        self.functions().iter().find(|f| f.name() == name)
    }

    /// Retrieves the module's functions.
    pub fn functions(&self) -> &[FunctionInfo] {
        if self.num_functions == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(self.functions, self.num_functions as usize) }
        }
    }
}

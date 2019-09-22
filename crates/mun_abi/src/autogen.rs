use crate::prelude::*;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::ffi::CStr;
use std::mem;
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

    /// Tries to downcast the `fn_ptr` to the specified function type.
    ///
    /// Returns an error message upon failure.
    pub fn downcast_fn2<A: Reflection, B: Reflection, Output: Reflection>(
        &self,
    ) -> Result<fn(A, B) -> Output, String> {
        let num_args = 2;

        let arg_types = self.arg_types();
        if arg_types.len() != num_args {
            return Err(format!(
                "Invalid number of arguments. Expected: {}. Found: {}.",
                arg_types.len(),
                num_args
            ));
        }

        if arg_types[0].guid != A::type_guid() {
            return Err(format!(
                "Invalid argument type for 'a'. Expected: {}. Found: {}.",
                arg_types[0].name(),
                A::type_name()
            ));
        }

        if arg_types[1].guid != B::type_guid() {
            return Err(format!(
                "Invalid argument type for 'b'. Expected: {}. Found: {}.",
                arg_types[1].name(),
                B::type_name()
            ));
        }

        if let Some(return_type) = self.return_type() {
            if return_type.guid != Output::type_guid() {
                return Err(format!(
                    "Invalid return type. Expected: {}. Found: {}",
                    return_type.name(),
                    Output::type_name(),
                ));
            }
        } else if <()>::type_guid() != Output::type_guid() {
            return Err(format!(
                "Invalid return type. Expected: {}. Found: {}",
                <()>::type_name(),
                Output::type_name(),
            ));
        }

        Ok(unsafe { mem::transmute(self.fn_ptr) })
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

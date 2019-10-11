#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use crate::prelude::*;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::ffi::{c_void, CStr};
use std::os::raw::c_char;
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

impl FunctionSignature {
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
}

impl ModuleInfo {
    /// Returns the module's full `path`.
    pub fn path(&self) -> &str {
        unsafe { CStr::from_ptr(self.path) }
            .to_str()
            .expect("Module path contains invalid UTF8")
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

    /// Retrieves the module's functions.
    pub fn functions(&self) -> &[FunctionInfo] {
        if self.num_functions == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(self.functions, self.num_functions as usize) }
        }
    }
}

impl DispatchTable {
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&mut *const c_void, &FunctionSignature)> {
        if self.num_entries == 0 {
            (&mut []).iter_mut().zip((&[]).iter())
        } else {
            let ptrs =
                unsafe { slice::from_raw_parts_mut(self.fn_ptrs, self.num_entries as usize) };
            let signatures =
                unsafe { slice::from_raw_parts(self.signatures, self.num_entries as usize) };

            ptrs.iter_mut().zip(signatures.iter())
        }
    }

    pub fn ptrs_mut(&mut self) -> &mut [*const c_void] {
        if self.num_entries == 0 {
            &mut []
        } else {
            unsafe { slice::from_raw_parts_mut(self.fn_ptrs, self.num_entries as usize) }
        }
    }

    pub fn signatures(&self) -> &[FunctionSignature] {
        if self.num_entries == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(self.signatures, self.num_entries as usize) }
        }
    }

    pub unsafe fn get_ptr_unchecked(&self, idx: u32) -> *const c_void {
        *self.fn_ptrs.offset(idx as isize)
    }

    pub fn get_ptr(&self, idx: u32) -> Option<*const c_void> {
        if idx < self.num_entries {
            Some(unsafe { self.get_ptr_unchecked(idx) })
        } else {
            None
        }
    }

    pub unsafe fn set_ptr_unchecked(&mut self, idx: u32, ptr: *const c_void) {
        *self.fn_ptrs.offset(idx as isize) = ptr;
    }

    pub fn set_ptr(&mut self, idx: u32, ptr: *const c_void) -> bool {
        if idx < self.num_entries {
            unsafe { self.set_ptr_unchecked(idx, ptr) };
            true
        } else {
            false
        }
    }
}

impl AssemblyInfo {
    pub fn dependencies(&self) -> impl Iterator<Item = &str> {
        let dependencies = if self.num_dependencies == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(self.dependencies, self.num_dependencies as usize) }
        };

        dependencies.iter().map(|d| {
            unsafe { CStr::from_ptr(*d) }
                .to_str()
                .expect("dependency path contains invalid UTF8")
        })
    }
}

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

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

    /// Returns a mutable reference to a function pointer, without doing bounds checking.
    ///
    /// This is generally not recommended, use with caution! Calling this method with an
    /// out-of-bounds index is _undefined behavior_ even if the resulting reference is not used.
    /// For a safe alternative see [get_ptr_mut](#method.get_ptr_mut).
    pub unsafe fn get_ptr_unchecked_mut(&self, idx: u32) -> &mut *const c_void {
        &mut *self.fn_ptrs.offset(idx as isize)
    }

    /// Returns a mutable reference to a function pointer at the given index, or `None` if out of
    /// bounds.
    pub fn get_ptr_mut(&self, idx: u32) -> Option<&mut *const c_void> {
        if idx < self.num_entries {
            Some(unsafe { self.get_ptr_unchecked_mut(idx) })
        } else {
            None
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::ptr;
    use std::os::raw::c_char;

    fn fake_type_info(name: &CStr) -> TypeInfo {
        TypeInfo {
            guid: FAKE_TYPE_GUID,
            name: name.as_ptr(),
        }
    }

    const FAKE_TYPE_GUID: Guid = Guid {
        b: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    };

    const FAKE_TYPE_NAME: &'static str = "type-name";

    #[test]
    fn test_type_info_name() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name);

        assert_eq!(type_info.name(), FAKE_TYPE_NAME);
    }

    #[test]
    fn test_type_info_eq() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name);

        assert_eq!(type_info, type_info);
    }

    fn fake_fn_signature(
        name: &CStr,
        arg_types: &[TypeInfo],
        return_type: Option<&TypeInfo>,
        privacy: Privacy,
    ) -> FunctionSignature {
        FunctionSignature {
            name: name.as_ptr(),
            arg_types: arg_types.as_ptr(),
            return_type: return_type.map_or(ptr::null(), |t| t as *const TypeInfo),
            num_arg_types: arg_types.len() as u16,
            privacy,
        }
    }

    const FAKE_FN_NAME: &'static str = "fn-name";

    #[test]
    fn test_fn_signature_name() {
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], None, Privacy::Public);

        assert_eq!(fn_signature.name(), FAKE_FN_NAME);
    }

    #[test]
    fn test_fn_signature_privacy() {
        let privacy = Privacy::Private;
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], None, privacy);

        assert_eq!(fn_signature.privacy(), privacy);
    }

    #[test]
    fn test_fn_signature_arg_types_none() {
        let arg_types = &[];
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, arg_types, None, Privacy::Public);

        assert_eq!(fn_signature.arg_types(), arg_types);
    }

    #[test]
    fn test_fn_signature_arg_types_some() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name);

        let arg_types = &[type_info];
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, arg_types, None, Privacy::Public);

        assert_eq!(fn_signature.arg_types(), arg_types);
    }

    #[test]
    fn test_fn_signature_return_type_none() {
        let return_type = None;
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], return_type, Privacy::Public);

        assert_eq!(fn_signature.return_type(), return_type);
    }

    #[test]
    fn test_fn_signature_return_type_some() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name);

        let return_type = Some(&type_info);
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], return_type, Privacy::Public);

        assert_eq!(fn_signature.return_type(), return_type);
    }

    fn fake_module_info(path: &CStr, functions: &[FunctionInfo]) -> ModuleInfo {
        ModuleInfo {
            path: path.as_ptr(),
            functions: functions.as_ptr(),
            num_functions: functions.len() as u32,
        }
    }

    const FAKE_MODULE_PATH: &'static str = "path::to::module";

    #[test]
    fn test_module_info_path() {
        let module_path = CString::new(FAKE_MODULE_PATH).expect("Invalid fake module path.");
        let module = fake_module_info(&module_path, &[]);

        assert_eq!(module.path(), FAKE_MODULE_PATH);
    }

    #[test]
    fn test_module_info_functions_none() {
        let functions = &[];
        let module_path = CString::new(FAKE_MODULE_PATH).expect("Invalid fake module path.");
        let module = fake_module_info(&module_path, functions);

        assert_eq!(module.functions().len(), functions.len());
    }

    #[test]
    fn test_module_info_functions_some() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name);

        let return_type = Some(&type_info);
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], return_type, Privacy::Public);

        let fn_info = FunctionInfo {
            signature: fn_signature,
            fn_ptr: ptr::null(),
        };

        let functions = &[fn_info];
        let module_path = CString::new(FAKE_MODULE_PATH).expect("Invalid fake module path.");
        let module = fake_module_info(&module_path, functions);

        let result = module.functions();
        assert_eq!(result.len(), functions.len());
        for (lhs, rhs) in result.iter().zip(functions.iter()) {
            assert_eq!(lhs.fn_ptr, rhs.fn_ptr);
            assert_eq!(lhs.signature.name(), rhs.signature.name());
            assert_eq!(lhs.signature.arg_types(), rhs.signature.arg_types());
            assert_eq!(lhs.signature.return_type(), rhs.signature.return_type());
            assert_eq!(lhs.signature.privacy(), rhs.signature.privacy());
        }
    }

    fn fake_dispatch_table(
        fn_signatures: &[FunctionSignature],
        fn_ptrs: &mut [*const c_void],
    ) -> DispatchTable {
        assert!(fn_signatures.len() == fn_ptrs.len());

        DispatchTable {
            signatures: fn_signatures.as_ptr(),
            fn_ptrs: fn_ptrs.as_mut_ptr(),
            num_entries: fn_signatures.len() as u32,
        }
    }

    #[test]
    fn test_dispatch_table_iter_mut_none() {
        let signatures = &[];
        let fn_ptrs = &mut [];
        let mut dispatch_table = fake_dispatch_table(signatures, fn_ptrs);

        let iter = fn_ptrs.iter_mut().zip(signatures.iter());
        assert_eq!(dispatch_table.iter_mut().count(), iter.count());
    }

    #[test]
    fn test_dispatch_table_iter_mut_some() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name);

        let return_type = Some(&type_info);
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], return_type, Privacy::Public);

        let signatures = &[fn_signature];
        let fn_ptrs = &mut [ptr::null()];
        let mut dispatch_table = fake_dispatch_table(signatures, fn_ptrs);

        let iter = fn_ptrs.iter_mut().zip(signatures.iter());
        assert_eq!(dispatch_table.iter_mut().count(), iter.len());

        for (lhs, rhs) in dispatch_table.iter_mut().zip(iter) {
            assert_eq!(lhs.0, rhs.0);
            assert_eq!(lhs.1.name(), rhs.1.name());
            assert_eq!(lhs.1.arg_types(), rhs.1.arg_types());
            assert_eq!(lhs.1.return_type(), rhs.1.return_type());
            assert_eq!(lhs.1.privacy(), rhs.1.privacy());
        }
    }

    #[test]
    fn test_dispatch_table_ptrs_mut() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name);

        let return_type = Some(&type_info);
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], return_type, Privacy::Public);

        let signatures = &[fn_signature];
        let fn_ptrs = &mut [ptr::null()];
        let mut dispatch_table = fake_dispatch_table(signatures, fn_ptrs);

        let result = dispatch_table.ptrs_mut();
        assert_eq!(result.len(), fn_ptrs.len());
        for (lhs, rhs) in result.iter().zip(fn_ptrs.iter()) {
            assert_eq!(lhs, rhs);
        }
    }

    #[test]
    fn test_dispatch_table_signatures() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name);

        let return_type = Some(&type_info);
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], return_type, Privacy::Public);

        let signatures = &[fn_signature];
        let fn_ptrs = &mut [ptr::null()];
        let dispatch_table = fake_dispatch_table(signatures, fn_ptrs);

        let result = dispatch_table.signatures();
        assert_eq!(result.len(), signatures.len());
        for (lhs, rhs) in result.iter().zip(signatures.iter()) {
            assert_eq!(lhs.name(), rhs.name());
            assert_eq!(lhs.arg_types(), rhs.arg_types());
            assert_eq!(lhs.return_type(), rhs.return_type());
            assert_eq!(lhs.privacy(), rhs.privacy());
        }
    }

    #[test]
    fn test_dispatch_table_get_ptr_unchecked() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name);

        let return_type = Some(&type_info);
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], return_type, Privacy::Public);

        let signatures = &[fn_signature];
        let fn_ptrs = &mut [ptr::null()];

        let dispatch_table = fake_dispatch_table(signatures, fn_ptrs);
        assert_eq!(unsafe { dispatch_table.get_ptr_unchecked(0) }, fn_ptrs[0]);
    }

    #[test]
    fn test_dispatch_table_get_ptr_none() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name);

        let return_type = Some(&type_info);
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], return_type, Privacy::Public);

        let signatures = &[fn_signature];
        let fn_ptrs = &mut [ptr::null()];

        let dispatch_table = fake_dispatch_table(signatures, fn_ptrs);
        assert_eq!(dispatch_table.get_ptr(1), None);
    }

    #[test]
    fn test_dispatch_table_get_ptr_some() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name);

        let return_type = Some(&type_info);
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], return_type, Privacy::Public);

        let signatures = &[fn_signature];
        let fn_ptrs = &mut [ptr::null()];

        let dispatch_table = fake_dispatch_table(signatures, fn_ptrs);
        assert_eq!(dispatch_table.get_ptr(0), Some(fn_ptrs[0]));
    }

    #[test]
    fn test_dispatch_table_get_ptr_unchecked_mut() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name);

        let return_type = Some(&type_info);
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], return_type, Privacy::Public);

        let signatures = &[fn_signature];
        let fn_ptrs = &mut [ptr::null()];

        let dispatch_table = fake_dispatch_table(signatures, fn_ptrs);
        assert_eq!(
            unsafe { dispatch_table.get_ptr_unchecked_mut(0) },
            &mut fn_ptrs[0]
        );
    }

    #[test]
    fn test_dispatch_table_get_ptr_mut_none() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name);

        let return_type = Some(&type_info);
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], return_type, Privacy::Public);

        let signatures = &[fn_signature];
        let fn_ptrs = &mut [ptr::null()];

        let dispatch_table = fake_dispatch_table(signatures, fn_ptrs);
        assert_eq!(dispatch_table.get_ptr_mut(1), None);
    }

    #[test]
    fn test_dispatch_table_get_ptr_mut_some() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_info(&type_name);

        let return_type = Some(&type_info);
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_signature = fake_fn_signature(&fn_name, &[], return_type, Privacy::Public);

        let signatures = &[fn_signature];
        let fn_ptrs = &mut [ptr::null()];

        let dispatch_table = fake_dispatch_table(signatures, fn_ptrs);
        assert_eq!(dispatch_table.get_ptr_mut(0), Some(&mut fn_ptrs[0]));
    }

    fn fake_assembly_info(
        symbols: ModuleInfo,
        dispatch_table: DispatchTable,
        dependencies: &[*const c_char],
    ) -> AssemblyInfo {
        AssemblyInfo {
            symbols,
            dispatch_table,
            dependencies: dependencies.as_ptr(),
            num_dependencies: dependencies.len() as u32,
        }
    }

    const FAKE_DEPENDENCY: &'static str = "path/to/dependency.dylib";

    #[test]
    fn test_assembly_info_dependencies() {
        let module_path = CString::new(FAKE_MODULE_PATH).expect("Invalid fake module path.");
        let module = fake_module_info(&module_path, &[]);

        let dispatch_table = fake_dispatch_table(&[], &mut []);

        let dependency = CString::new(FAKE_DEPENDENCY).expect("Invalid fake dependency.");
        let dependencies = &[dependency.as_ptr()];
        let assembly = fake_assembly_info(module, dispatch_table, dependencies);

        assert_eq!(assembly.dependencies().count(), dependencies.len());
        for (lhs, rhs) in assembly.dependencies().zip([FAKE_DEPENDENCY].iter()) {
            assert_eq!(lhs, *rhs)
        }
    }
}

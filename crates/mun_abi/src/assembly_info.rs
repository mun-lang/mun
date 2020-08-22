use crate::{DispatchTable, ModuleInfo, TypeInfo};
use std::{ffi::CStr, os::raw::c_char, slice, str};

/// Represents an assembly declaration.
#[repr(C)]
pub struct AssemblyInfo {
    /// Symbols of the top-level module
    pub symbols: ModuleInfo,
    /// Dispatch table
    pub dispatch_table: DispatchTable,
    /// Paths to assembly dependencies
    pub(crate) dependencies: *const *const c_char,
    /// Assembly types
    pub(crate) types: *const *const TypeInfo,
    /// Number of dependencies
    pub num_dependencies: u32,
    /// Number of types
    pub num_types: u32,
}

impl AssemblyInfo {
    /// Returns an iterator over the assembly's dependencies.
    pub fn dependencies(&self) -> impl Iterator<Item = &str> {
        let dependencies = if self.num_dependencies == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(self.dependencies, self.num_dependencies as usize) }
        };

        dependencies
            .iter()
            .map(|d| unsafe { str::from_utf8_unchecked(CStr::from_ptr(*d).to_bytes()) })
    }

    /// Returns the assembly's types.
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

unsafe impl Send for AssemblyInfo {}
unsafe impl Sync for AssemblyInfo {}

#[cfg(test)]
mod tests {
    use crate::{
        test_utils::{
            fake_assembly_info, fake_dispatch_table, fake_module_info, fake_struct_info,
            fake_struct_type_info, FAKE_DEPENDENCY, FAKE_MODULE_PATH, FAKE_STRUCT_NAME,
        },
        TypeGroup,
    };
    use std::{ffi::CString, mem};

    #[test]
    fn test_assembly_info_dependencies_none() {
        let module_path = CString::new(FAKE_MODULE_PATH).expect("Invalid fake module path.");
        let module = fake_module_info(&module_path, &[]);

        let dispatch_table = fake_dispatch_table(&[], &mut []);

        let dependencies = &[];
        let assembly = fake_assembly_info(module, dispatch_table, dependencies, &[]);

        assert_eq!(assembly.dependencies().count(), dependencies.len());
    }

    #[test]
    fn test_assembly_info_dependencies_some() {
        let module_path = CString::new(FAKE_MODULE_PATH).expect("Invalid fake module path.");
        let module = fake_module_info(&module_path, &[]);

        let dispatch_table = fake_dispatch_table(&[], &mut []);

        let dependency = CString::new(FAKE_DEPENDENCY).expect("Invalid fake dependency.");
        let dependencies = &[dependency.as_ptr()];
        let assembly = fake_assembly_info(module, dispatch_table, dependencies, &[]);

        assert_eq!(assembly.dependencies().count(), dependencies.len());
        for (lhs, rhs) in assembly.dependencies().zip([FAKE_DEPENDENCY].iter()) {
            assert_eq!(lhs, *rhs)
        }
    }

    #[test]
    fn test_assembly_info_types_none() {
        let module_path = CString::new(FAKE_MODULE_PATH).expect("Invalid fake module path.");
        let module = fake_module_info(&module_path, &[]);

        let dispatch_table = fake_dispatch_table(&[], &mut []);

        let types = &[];
        let assembly = fake_assembly_info(module, dispatch_table, &[], types);

        assert_eq!(assembly.types().iter().count(), types.len());
    }

    #[test]
    fn test_assembly_info_types_some() {
        let module_path = CString::new(FAKE_MODULE_PATH).expect("Invalid fake module path.");
        let module = fake_module_info(&module_path, &[]);

        let dispatch_table = fake_dispatch_table(&[], &mut []);

        let struct_name = CString::new(FAKE_STRUCT_NAME).expect("Invalid fake struct name");
        let struct_info = fake_struct_info(&[], &[], &[], Default::default());
        let struct_type_info = fake_struct_type_info(&struct_name, struct_info, 1, 1);

        let types = &[unsafe { mem::transmute(&struct_type_info) }];
        let assembly = fake_assembly_info(module, dispatch_table, &[], types);

        assert_eq!(assembly.types().iter().count(), types.len());

        let result_types = assembly.types();
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

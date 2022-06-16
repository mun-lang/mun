use crate::{DispatchTable, ModuleInfo, TypeLut};
use std::{ffi::CStr, os::raw::c_char, slice, str};

/// Represents an assembly declaration.
#[repr(C)]
pub struct AssemblyInfo<'a> {
    /// Symbols of the top-level module
    pub symbols: ModuleInfo<'a>,
    /// Function dispatch table
    pub dispatch_table: DispatchTable<'a>,
    /// Type lookup table
    pub type_lut: TypeLut<'a>,
    /// Paths to assembly dependencies
    pub(crate) dependencies: *const *const c_char,
    /// Number of dependencies
    pub num_dependencies: u32,
}

impl<'a> AssemblyInfo<'a> {
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
}

unsafe impl<'a> Send for AssemblyInfo<'a> {}
unsafe impl<'a> Sync for AssemblyInfo<'a> {}

#[cfg(test)]
mod tests {
    use crate::test_utils::{
        fake_assembly_info, fake_dispatch_table, fake_module_info, fake_type_lut, FAKE_DEPENDENCY,
        FAKE_MODULE_PATH,
    };
    use std::ffi::CString;

    #[test]
    fn test_assembly_info_dependencies() {
        let module_path = CString::new(FAKE_MODULE_PATH).expect("Invalid fake module path.");
        let module = fake_module_info(&module_path, &[], &[]);

        let dispatch_table = fake_dispatch_table(&[], &mut []);
        let type_lut = fake_type_lut(&[], &mut [], &[]);

        let dependency = CString::new(FAKE_DEPENDENCY).expect("Invalid fake dependency.");
        let dependencies = &[dependency.as_ptr()];
        let assembly = fake_assembly_info(module, dispatch_table, type_lut, dependencies);

        assert_eq!(assembly.dependencies().count(), dependencies.len());
        for (lhs, rhs) in assembly.dependencies().zip([FAKE_DEPENDENCY].iter()) {
            assert_eq!(lhs, *rhs)
        }
    }
}

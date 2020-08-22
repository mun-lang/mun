mod temp_library;

use std::{ffi::c_void, path::Path};

pub use temp_library::TempLibrary;

pub struct MunLibrary(TempLibrary);

impl MunLibrary {
    pub fn new(library_path: &Path) -> Result<Self, anyhow::Error> {
        let library = TempLibrary::new(library_path)?;

        // Verify that the `*.munlib` contains all required functions
        let _get_abi_version_fn: libloading::Symbol<'_, extern "C" fn() -> u32> =
            unsafe { library.library().get(abi::GET_VERSION_FN_NAME.as_bytes()) }?;

        let _get_info_fn: libloading::Symbol<'_, extern "C" fn() -> abi::AssemblyInfo> =
            unsafe { library.library().get(abi::GET_INFO_FN_NAME.as_bytes()) }?;

        let _set_allocator_handle_fn: libloading::Symbol<'_, extern "C" fn(*mut c_void)> = unsafe {
            library
                .library()
                .get(abi::SET_ALLOCATOR_HANDLE_FN_NAME.as_bytes())
        }?;

        Ok(MunLibrary(library))
    }

    pub fn into_inner(self) -> TempLibrary {
        self.0
    }

    pub fn get_abi_version(&self) -> u32 {
        let get_abi_version_fn: libloading::Symbol<'_, extern "C" fn() -> u32> = unsafe {
            self.0
                .library()
                .get(abi::GET_VERSION_FN_NAME.as_bytes())
                .unwrap()
        };

        get_abi_version_fn()
    }

    pub fn get_info(&self) -> abi::AssemblyInfo {
        let get_info_fn: libloading::Symbol<'_, extern "C" fn() -> abi::AssemblyInfo> = unsafe {
            self.0
                .library()
                .get(abi::GET_INFO_FN_NAME.as_bytes())
                .unwrap()
        };

        get_info_fn()
    }

    pub fn set_allocator_handle(&mut self, allocator_ptr: *mut c_void) {
        let set_allocator_handle_fn: libloading::Symbol<'_, extern "C" fn(*mut c_void)> = unsafe {
            self.0
                .library()
                .get(abi::SET_ALLOCATOR_HANDLE_FN_NAME.as_bytes())
                .unwrap()
        };

        set_allocator_handle_fn(allocator_ptr);
    }
}

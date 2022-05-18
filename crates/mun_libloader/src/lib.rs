mod temp_library;

use std::{ffi::c_void, path::Path};

pub use temp_library::TempLibrary;

pub struct MunLibrary(TempLibrary);

impl MunLibrary {
    /// Loads a munlib library from disk.
    ///
    /// # Safety
    ///
    /// A munlib is simply a shared object. When a library is loaded, initialisation routines
    /// contained within it are executed. For the purposes of safety, the execution of these
    /// routines is conceptually the same calling an unknown foreign function and may impose
    /// arbitrary requirements on the caller for the call to be sound.
    ///
    /// Additionally, the callers of this function must also ensure that execution of the
    /// termination routines contained within the library is safe as well. These routines may be
    /// executed when the library is unloaded.
    ///
    /// See [`libloading::Library::new`] for more information.
    pub unsafe fn new(library_path: &Path) -> Result<Self, anyhow::Error> {
        // Although loading a library is technically unsafe, we assume here that this is not the
        // case for munlibs.
        let library = TempLibrary::new(library_path)?;

        // Verify that the `*.munlib` contains all required functions. Note that this is an unsafe
        // operation because the loaded symbols don't actually contain type information. Casting
        // is therefore unsafe.
        let _get_abi_version_fn: libloading::Symbol<'_, extern "C" fn() -> u32> =
            library.library().get(abi::GET_VERSION_FN_NAME.as_bytes())?;

        let _get_info_fn: libloading::Symbol<'_, extern "C" fn() -> abi::AssemblyInfo> =
            library.library().get(abi::GET_INFO_FN_NAME.as_bytes())?;

        let _set_allocator_handle_fn: libloading::Symbol<'_, extern "C" fn(*mut c_void)> = library
            .library()
            .get(abi::SET_ALLOCATOR_HANDLE_FN_NAME.as_bytes())?;

        Ok(MunLibrary(library))
    }

    pub fn into_inner(self) -> TempLibrary {
        self.0
    }

    /// Returns the ABI version of this mun library.
    ///
    /// # Safety
    ///
    /// This operations executes a function in the munlib. There is no guarantee that the execution
    /// of the function wont result in undefined behavior.
    pub unsafe fn get_abi_version(&self) -> u32 {
        let get_abi_version_fn: libloading::Symbol<'_, extern "C" fn() -> u32> = self
            .0
            .library()
            .get(abi::GET_VERSION_FN_NAME.as_bytes())
            .unwrap();

        get_abi_version_fn()
    }

    /// Returns the assembly info exported by the shared object.
    ///
    /// # Safety
    ///
    /// This operations executes a function in the munlib. There is no guarantee that the execution
    /// of the function wont result in undefined behavior.
    pub unsafe fn get_info(&self) -> abi::AssemblyInfo {
        let get_info_fn: libloading::Symbol<'_, extern "C" fn() -> abi::AssemblyInfo> = self
            .0
            .library()
            .get(abi::GET_INFO_FN_NAME.as_bytes())
            .unwrap();

        get_info_fn()
    }

    /// Stores the allocator handle inside the shared object. This is used by the internals of the
    /// library to be able to allocate memory.
    ///
    /// # Safety
    ///
    /// This operations executes a function in the munlib. There is no guarantee that the execution
    /// of the function wont result in undefined behavior.
    pub unsafe fn set_allocator_handle(&mut self, allocator_ptr: *mut c_void) {
        let set_allocator_handle_fn: libloading::Symbol<'_, extern "C" fn(*mut c_void)> = self
            .0
            .library()
            .get(abi::SET_ALLOCATOR_HANDLE_FN_NAME.as_bytes())
            .unwrap();

        set_allocator_handle_fn(allocator_ptr);
    }
}

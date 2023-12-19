//! Exposes function information using the C ABI.

use mun_capi_utils::error::ErrorHandle;
use mun_capi_utils::{mun_error_try, try_deref_mut};
use mun_memory::ffi::{Type, Types};
use std::{
    ffi::{c_void, CString},
    os::raw::c_char,
    ptr,
    sync::Arc,
};

/// Describes a `Function` accessible from a Mun [`super::runtime::Runtime`].
///
/// An instance of `Function` shares ownership of the underlying data. To create a copy of the
/// `Function` object call [`mun_function_add_reference`] to make sure the number of references to
/// the data is properly tracked. Calling [`mun_function_release`] signals the runtime that the data
/// is no longer referenced through the specified object. When all references are released the
/// underlying data is deallocated.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Function(pub *const c_void);

impl Function {
    /// Returns a Self containing nulls.
    pub fn null() -> Self {
        Self(ptr::null())
    }

    /// Returns a reference to the data that this instance is referencing.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the internal pointers point to a valid
    /// [`mun_runtime::FunctionDefinition`].
    pub unsafe fn inner(&self) -> Result<&mun_runtime::FunctionDefinition, &'static str> {
        (self.0 as *const mun_runtime::FunctionDefinition)
            .as_ref()
            .ok_or("null pointer")
    }
}

impl From<Arc<mun_runtime::FunctionDefinition>> for Function {
    fn from(def: Arc<mun_runtime::FunctionDefinition>) -> Self {
        Function(Arc::into_raw(def).cast())
    }
}

/// Notifies the runtime an additional references exists to the function. This ensures that the data
/// is kept alive even if [`mun_function_release`] is called for the existing references. Only
/// after all references have been released can the underlying data be deallocated.
///
/// # Safety
///
/// This function might be unsafe if the underlying data has already been deallocated by a previous
/// call to [`mun_function_release`].
#[no_mangle]
pub unsafe extern "C" fn mun_function_add_reference(function: Function) -> ErrorHandle {
    if function.0.is_null() {
        return ErrorHandle::new("invalid argument 'function': null pointer");
    }

    Arc::increment_strong_count(function.0);
    ErrorHandle::default()
}

/// Notifies the runtime that one of the references to the function is no longer in use. The data
/// may not immediately be destroyed. Only after all references have been released can the
/// underlying data be deallocated.
///
/// # Safety
///
/// This function might be unsafe if the underlying data has been deallocated by a previous call
/// to [`mun_function_release`].
#[no_mangle]
pub unsafe extern "C" fn mun_function_release(function: Function) -> ErrorHandle {
    if function.0.is_null() {
        return ErrorHandle::new("invalid argument 'function': null pointer");
    }

    Arc::decrement_strong_count(function.0);
    ErrorHandle::default()
}

/// Retrieves the function's function pointer.
///
/// # Safety
///
/// This function might be unsafe if the underlying data has been deallocated by a previous call
/// to [`mun_function_release`].
#[no_mangle]
pub unsafe extern "C" fn mun_function_fn_ptr(
    function: Function,
    ptr: *mut *const c_void,
) -> ErrorHandle {
    let function = mun_error_try!(function
        .inner()
        .map_err(|e| format!("invalid argument 'function': {e}")));
    let ptr = try_deref_mut!(ptr);
    *ptr = function.fn_ptr;
    ErrorHandle::default()
}

/// Retrieves the function's name.
///
/// If the function is successful, the caller is responsible for calling [`mun_string_destroy`] on
/// the return pointer.
///
/// # Safety
///
/// This function might be unsafe if the underlying data has been deallocated by a previous call
/// to [`mun_function_release`].
#[no_mangle]
pub unsafe extern "C" fn mun_function_name(
    function: Function,
    name: *mut *const c_char,
) -> ErrorHandle {
    let function = mun_error_try!(function
        .inner()
        .map_err(|e| format!("invalid argument 'function': {e}")));
    let name = try_deref_mut!(name);
    *name = CString::new(function.prototype.name.clone())
        .unwrap()
        .into_raw() as *const _;
    ErrorHandle::default()
}

/// Retrieves the function's argument types.
///
/// If successful, ownership of the [`Types`] is transferred to the caller. It must be deallocated
/// with a call to [`mun_types_destroy`].
///
/// # Safety
///
///
/// This function might be unsafe if the underlying data has been deallocated by a previous call
/// to [`mun_function_release`].
#[no_mangle]
pub unsafe extern "C" fn mun_function_argument_types(
    function: Function,
    arg_types: *mut Types,
) -> ErrorHandle {
    let function = mun_error_try!(function
        .inner()
        .map_err(|e| format!("invalid argument 'function': {e}")));
    let arg_types = try_deref_mut!(arg_types);
    *arg_types = function
        .prototype
        .signature
        .arg_types
        .iter()
        .map(|ty| ty.clone().into())
        .collect::<Vec<_>>()
        .into();
    ErrorHandle::default()
}

/// Retrieves the function's return type.
///
/// Ownership of the [`Type`] is transferred to the called. It must be released with a call to
/// [`mun_type_release`].
///
/// # Safety
///
/// This function might be unsafe if the underlying data has been deallocated by a previous call
/// to [`mun_function_release`].
#[no_mangle]
pub unsafe extern "C" fn mun_function_return_type(
    function: Function,
    ty: *mut Type,
) -> ErrorHandle {
    let function = mun_error_try!(function
        .inner()
        .map_err(|e| format!("invalid argument 'function': {e}")));
    let ty = try_deref_mut!(ty);
    *ty = function.prototype.signature.return_type.clone().into();
    ErrorHandle::default()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use mun_capi_utils::{
        assert_error_snapshot, assert_getter1, mun_string_destroy, try_convert_c_string,
    };
    use mun_memory::ffi::{mun_type_equal, mun_types_destroy};
    use mun_memory::HasStaticType;
    use std::mem::ManuallyDrop;
    use std::{mem::MaybeUninit, slice, sync::Arc};

    #[test]
    fn test_function_release_invalid_fn_info() {
        assert_error_snapshot!(
            unsafe { mun_function_release(Function::null()) },
            @r#""invalid argument \'function\': null pointer""#);
    }

    #[test]
    fn test_function_add_reference_invalid_fn_info() {
        assert_error_snapshot!(
            unsafe { mun_function_add_reference(Function::null()) },
            @r#""invalid argument \'function\': null pointer""#);
    }

    #[test]
    fn test_function_release_strong_count() {
        let fn_def = mun_runtime::FunctionDefinition::builder("foo").finish();
        let ffi_function: Function = fn_def.clone().into();

        let strong_count = Arc::strong_count(&fn_def);
        assert!(strong_count == 2);

        assert!(unsafe { mun_function_release(ffi_function) }.is_ok());

        // This works because the Arc is not shared between threads because it's local to the
        // runtime created in this test
        assert_eq!(Arc::strong_count(&fn_def), strong_count - 1);
    }

    #[test]
    fn test_function_add_reference_strong_count() {
        let function: Function = mun_runtime::FunctionDefinition::builder("foo")
            .finish()
            .into();

        let fn_info_arc = ManuallyDrop::new(unsafe {
            Arc::from_raw(function.0 as *const mun_runtime::FunctionDefinition)
        });
        let strong_count = Arc::strong_count(&fn_info_arc);
        assert!(strong_count > 0);

        assert!(unsafe { mun_function_add_reference(function) }.is_ok());

        // This works because the Arc is not shared between threads because it's local to the
        // runtime created in this test
        assert_eq!(Arc::strong_count(&fn_info_arc), strong_count + 1);
    }

    #[test]
    fn test_function_invalid_fn_info() {
        let function = mun_runtime::FunctionDefinition::builder("foo")
            .finish()
            .into();

        let mut ptr = MaybeUninit::uninit();
        assert_error_snapshot!(
            unsafe { mun_function_fn_ptr(Function::null(), ptr.as_mut_ptr()) },
            @r#""invalid argument \'function\': null pointer""#);
        assert_error_snapshot!(
            unsafe { mun_function_fn_ptr(function, ptr::null_mut()) },
            @r#""invalid argument \'ptr\': null pointer""#);

        assert!(unsafe { mun_function_release(function) }.is_ok());
    }

    #[test]
    fn test_function_fn_ptr() {
        let invalid_fn_ptr = 0xDEAD as *const c_void;
        let function = mun_runtime::FunctionDefinition::builder("foo")
            .set_ptr(invalid_fn_ptr)
            .finish()
            .into();

        assert_getter1!(mun_function_fn_ptr(function, fn_ptr));
        assert_eq!(fn_ptr, invalid_fn_ptr);

        assert!(unsafe { mun_function_release(function) }.is_ok());
    }

    #[test]
    fn test_function_name_invalid_fn_info() {
        let function = mun_runtime::FunctionDefinition::builder("foo")
            .finish()
            .into();

        let mut ptr = MaybeUninit::uninit();
        assert_error_snapshot!(
            unsafe { mun_function_name(Function::null(), ptr.as_mut_ptr()) },
            @r#""invalid argument \'function\': null pointer""#);
        assert_error_snapshot!(
            unsafe { mun_function_name(function, ptr::null_mut()) },
            @r#""invalid argument \'name\': null pointer""#);

        assert!(unsafe { mun_function_release(function) }.is_ok());
    }

    #[test]
    fn test_function_name() {
        let function = mun_runtime::FunctionDefinition::builder("foo")
            .finish()
            .into();

        assert_getter1!(mun_function_name(function, name));
        assert_ne!(name, ptr::null());

        let name_str = unsafe { try_convert_c_string(name) }.expect("invalid name");
        assert_eq!(name_str, "foo");

        unsafe { mun_string_destroy(name) };
        assert!(unsafe { mun_function_release(function) }.is_ok());
    }

    #[test]
    fn test_function_argument_types_invalid_fn_info() {
        let function = mun_runtime::FunctionDefinition::builder("foo")
            .finish()
            .into();

        let mut ptr = MaybeUninit::uninit();
        assert_error_snapshot!(unsafe {
            mun_function_argument_types(Function::null(), ptr.as_mut_ptr())
        }, @r#""invalid argument \'function\': null pointer""#);
        assert_error_snapshot!(
            unsafe { mun_function_argument_types(function, ptr::null_mut()) },
            @r#""invalid argument \'arg_types\': null pointer""#);

        assert!(unsafe { mun_function_release(function) }.is_ok());
    }

    #[test]
    fn test_function_argument_types_none() {
        let function = mun_runtime::FunctionDefinition::builder("foo")
            .finish()
            .into();

        assert_getter1!(mun_function_argument_types(function, arg_types));
        assert_eq!(arg_types.types, ptr::null());
        assert_eq!(arg_types.count, 0);

        assert!(unsafe { mun_types_destroy(arg_types) }.is_ok());
        assert!(unsafe { mun_function_release(function) }.is_ok());
    }

    #[test]
    fn test_function_argument_types_some() {
        let function = mun_runtime::FunctionDefinition::builder("foo")
            .add_argument(i32::type_info().clone())
            .add_argument(i32::type_info().clone())
            .finish()
            .into();

        assert_getter1!(mun_function_argument_types(function, arg_types));
        assert_eq!(arg_types.count, 2);

        for arg_type in unsafe { slice::from_raw_parts(arg_types.types, arg_types.count) } {
            assert!(unsafe { mun_type_equal(*arg_type, i32::type_info().clone().into()) });
        }

        assert!(unsafe { mun_types_destroy(arg_types) }.is_ok());
        assert!(unsafe { mun_function_release(function) }.is_ok());
    }

    #[test]
    fn test_function_return_type_invalid_fn_info() {
        let function = mun_runtime::FunctionDefinition::builder("foo")
            .finish()
            .into();

        let mut ptr = MaybeUninit::uninit();
        assert_error_snapshot!(
            unsafe { mun_function_return_type(Function::null(), ptr.as_mut_ptr()) },
            @r#""invalid argument \'function\': null pointer""#);
        assert_error_snapshot!(
            unsafe { mun_function_return_type(function, ptr::null_mut()) },
            @r#""invalid argument \'ty\': null pointer""#);

        assert!(unsafe { mun_function_release(function) }.is_ok());
    }

    #[test]
    fn test_function_return_type_none() {
        let function = mun_runtime::FunctionDefinition::builder("foo")
            .finish()
            .into();

        assert_getter1!(mun_function_return_type(function, return_type));

        assert!(unsafe { mun_type_equal(return_type, <()>::type_info().clone().into()) });
    }

    #[test]
    fn test_function_return_type_some() {
        let function = mun_runtime::FunctionDefinition::builder("foo")
            .set_return_type(i32::type_info().clone())
            .finish()
            .into();

        assert_getter1!(mun_function_return_type(function, return_type));

        assert!(unsafe { mun_type_equal(return_type, i32::type_info().clone().into()) });
    }
}

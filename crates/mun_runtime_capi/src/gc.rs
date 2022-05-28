//! Exposes Mun garbage collection.

use std::{ffi::c_void, sync::Arc};

use crate::{error::ErrorHandle, runtime::RuntimeHandle, type_info::TypeInfoHandle};
use memory::TypeInfo;
use runtime::Runtime;

pub use memory::gc::GcPtr;

/// Allocates an object in the runtime of the given `type_info`. If successful, `obj` is set,
/// otherwise a non-zero error handle is returned.
///
/// If a non-zero error handle is returned, it must be manually destructed using
/// [`mun_error_destroy`].
///
/// # Safety
///
/// This function receives raw pointers as parameters. If any of the arguments is a null pointer,
/// an error will be returned. Passing pointers to invalid data, will lead to undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn mun_gc_alloc(
    runtime: RuntimeHandle,
    type_info: TypeInfoHandle,
    obj: *mut GcPtr,
) -> ErrorHandle {
    let runtime = match (runtime.0 as *mut Runtime).as_ref() {
        Some(runtime) => runtime,
        None => return ErrorHandle::new("Invalid argument: 'runtime' is null pointer."),
    };

    if type_info.0.is_null() {
        return ErrorHandle::new("Invalid argument: 'type_info' is null pointer.");
    }

    let obj = match obj.as_mut() {
        Some(obj) => obj,
        None => return ErrorHandle::new("Invalid argument: 'obj' is null pointer."),
    };

    let type_info = Arc::from_raw(type_info.0 as *const TypeInfo);

    *obj = runtime.gc().alloc(&type_info);

    std::mem::forget(type_info);

    ErrorHandle::default()
}

/// Retrieves the `type_info` for the specified `obj` from the runtime. If successful, `type_info`
/// is set, otherwise a non-zero error handle is returned.
///
/// If a non-zero error handle is returned, it must be manually destructed using
/// [`mun_error_destroy`].
///
/// # Safety
///
/// This function receives raw pointers as parameters. If any of the arguments is a null pointer,
/// an error will be returned. Passing pointers to invalid data, will lead to undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn mun_gc_ptr_type(
    handle: RuntimeHandle,
    obj: GcPtr,
    type_info: *mut TypeInfoHandle,
) -> ErrorHandle {
    let runtime = match (handle.0 as *mut Runtime).as_ref() {
        Some(runtime) => runtime,
        None => return ErrorHandle::new("Invalid argument: 'runtime' is null pointer."),
    };

    let type_info = match type_info.as_mut() {
        Some(type_info) => type_info,
        None => return ErrorHandle::new("Invalid argument: 'type_info' is null pointer."),
    };

    type_info.0 = Arc::into_raw(runtime.gc().ptr_type(obj)) as *const c_void;

    ErrorHandle::default()
}

/// Roots the specified `obj`, which keeps it and objects it references alive. Objects marked as
/// root, must call `mun_gc_unroot` before they can be collected. An object can be rooted multiple
/// times, but you must make sure to call `mun_gc_unroot` an equal number of times before the
/// object can be collected. If successful, `obj` has been rooted, otherwise a non-zero error handle
/// is returned.
///
/// If a non-zero error handle is returned, it must be manually destructed using
/// [`mun_error_destroy`].
///
/// # Safety
///
/// This function receives raw pointers as parameters. If any of the arguments is a null pointer,
/// an error will be returned. Passing pointers to invalid data, will lead to undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn mun_gc_root(handle: RuntimeHandle, obj: GcPtr) -> ErrorHandle {
    let runtime = match (handle.0 as *mut Runtime).as_ref() {
        Some(runtime) => runtime,
        None => return ErrorHandle::new("Invalid argument: 'runtime' is null pointer."),
    };

    runtime.gc().root(obj);
    ErrorHandle::default()
}

/// Unroots the specified `obj`, potentially allowing it and objects it references to be
/// collected. An object can be rooted multiple times, so you must make sure to call `mun_gc_unroot`
/// the same number of times as `mun_gc_root` was called before the object can be collected. If
/// successful, `obj` has been unrooted, otherwise a non-zero error handle is returned.
///
/// If a non-zero error handle is returned, it must be manually destructed using
/// [`mun_error_destroy`].
///
/// # Safety
///
/// This function receives raw pointers as parameters. If any of the arguments is a null pointer,
/// an error will be returned. Passing pointers to invalid data, will lead to undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn mun_gc_unroot(handle: RuntimeHandle, obj: GcPtr) -> ErrorHandle {
    let runtime = match (handle.0 as *mut Runtime).as_ref() {
        Some(runtime) => runtime,
        None => return ErrorHandle::new("Invalid argument: 'runtime' is null pointer."),
    };

    runtime.gc().unroot(obj);
    ErrorHandle::default()
}

/// Collects all memory that is no longer referenced by rooted objects. If successful, `reclaimed`
/// is set, otherwise a non-zero error handle is returned. If `reclaimed` is `true`, memory was
/// reclaimed, otherwise nothing happend. This behavior will likely change in the future.
///
/// If a non-zero error handle is returned, it must be manually destructed using
/// [`mun_error_destroy`].
///
/// # Safety
///
/// This function receives raw pointers as parameters. If any of the arguments is a null pointer,
/// an error will be returned. Passing pointers to invalid data, will lead to undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn mun_gc_collect(
    handle: RuntimeHandle,
    reclaimed: *mut bool,
) -> ErrorHandle {
    let runtime = match (handle.0 as *mut Runtime).as_ref() {
        Some(runtime) => runtime,
        None => return ErrorHandle::new("Invalid argument: 'runtime' is null pointer."),
    };

    let reclaimed = match reclaimed.as_mut() {
        Some(reclaimed) => reclaimed,
        None => return ErrorHandle::new("Invalid argument: 'reclaimed' is null pointer."),
    };

    *reclaimed = runtime.gc_collect();
    ErrorHandle::default()
}

#[cfg(test)]
mod tests {
    use memory::gc::{HasIndirectionPtr, RawGcPtr};

    use super::*;
    use crate::{
        error::mun_error_destroy, runtime::mun_runtime_get_type_info_by_name, test_invalid_runtime,
        test_util::TestDriver,
    };
    use std::{
        ffi::{CStr, CString},
        mem::{self, MaybeUninit},
        ptr,
    };

    test_invalid_runtime!(
        gc_alloc(TypeInfoHandle::null(), ptr::null_mut()),
        gc_ptr_type(mem::zeroed::<GcPtr>(), ptr::null_mut()),
        gc_root(mem::zeroed::<GcPtr>()),
        gc_unroot(mem::zeroed::<GcPtr>()),
        gc_collect(ptr::null_mut())
    );

    #[test]
    fn test_gc_alloc_invalid_obj() {
        let driver = TestDriver::new(
            r#"
        pub struct Foo;
    "#,
        );

        let type_name = CString::new("Foo").expect("Invalid type name.");
        let mut has_type = false;
        let mut type_info = TypeInfoHandle::null();

        let handle = unsafe {
            mun_runtime_get_type_info_by_name(
                driver.runtime,
                type_name.as_ptr(),
                &mut has_type as *mut bool,
                &mut type_info as *mut TypeInfoHandle,
            )
        };
        assert_eq!(handle.0, ptr::null());

        let handle = unsafe { mun_gc_alloc(driver.runtime, type_info, ptr::null_mut()) };
        assert_ne!(handle.0, ptr::null());

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'obj' is null pointer."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_gc_alloc() {
        let driver = TestDriver::new(
            r#"
        pub struct Foo;
    "#,
        );

        let type_name = CString::new("Foo").expect("Invalid type name.");
        let mut has_type = false;
        let mut type_info = TypeInfoHandle::null();

        let handle = unsafe {
            mun_runtime_get_type_info_by_name(
                driver.runtime,
                type_name.as_ptr(),
                &mut has_type as *mut bool,
                &mut type_info as *mut TypeInfoHandle,
            )
        };
        assert_eq!(handle.0, ptr::null());

        let mut obj = MaybeUninit::uninit();
        let handle = unsafe { mun_gc_alloc(driver.runtime, type_info, obj.as_mut_ptr()) };
        assert_eq!(handle.0, ptr::null());

        let obj = unsafe { obj.assume_init() };
        assert_ne!(unsafe { obj.deref::<u8>() }, ptr::null());

        let mut reclaimed = false;
        let handle = unsafe { mun_gc_collect(driver.runtime, &mut reclaimed as *mut _) };
        assert_eq!(handle.0, ptr::null());
    }

    #[test]
    fn test_gc_ptr_type_invalid_type_info() {
        let driver = TestDriver::new(
            r#"
        pub struct Foo;
    "#,
        );

        let handle = unsafe {
            let raw_ptr: RawGcPtr = ptr::null();
            mun_gc_ptr_type(driver.runtime, raw_ptr.into(), ptr::null_mut())
        };

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'type_info' is null pointer."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_gc_ptr_type() {
        let driver = TestDriver::new(
            r#"
        pub struct Foo;
    "#,
        );

        let type_name = CString::new("Foo").expect("Invalid type name.");
        let mut has_type = false;
        let mut type_info = TypeInfoHandle::null();

        let handle = unsafe {
            mun_runtime_get_type_info_by_name(
                driver.runtime,
                type_name.as_ptr(),
                &mut has_type as *mut bool,
                &mut type_info as *mut TypeInfoHandle,
            )
        };
        assert_eq!(handle.0, ptr::null());

        let mut obj = MaybeUninit::uninit();
        let handle = unsafe { mun_gc_alloc(driver.runtime, type_info, obj.as_mut_ptr()) };
        assert_eq!(handle.0, ptr::null());

        let obj = unsafe { obj.assume_init() };
        assert_ne!(unsafe { obj.deref::<u8>() }, ptr::null());

        let mut ty = MaybeUninit::uninit();
        let handle = unsafe { mun_gc_ptr_type(driver.runtime, obj, ty.as_mut_ptr()) };
        assert_eq!(handle.0, ptr::null());

        let ty = unsafe { ty.assume_init() };
        assert_eq!(type_info.0, ty.0);

        let mut reclaimed = false;
        let handle = unsafe { mun_gc_collect(driver.runtime, &mut reclaimed as *mut _) };
        assert_eq!(handle.0, ptr::null());
        assert!(reclaimed);
    }

    #[test]
    fn test_gc_rooting() {
        let driver = TestDriver::new(
            r#"
        pub struct Foo;
    "#,
        );

        let type_name = CString::new("Foo").expect("Invalid type name.");
        let mut has_type = false;
        let mut type_info = TypeInfoHandle::null();

        let handle = unsafe {
            mun_runtime_get_type_info_by_name(
                driver.runtime,
                type_name.as_ptr(),
                &mut has_type as *mut bool,
                &mut type_info as *mut TypeInfoHandle,
            )
        };
        assert_eq!(handle.0, ptr::null());

        let mut obj = MaybeUninit::uninit();
        let handle = unsafe { mun_gc_alloc(driver.runtime, type_info, obj.as_mut_ptr()) };
        assert_eq!(handle.0, ptr::null());

        let obj = unsafe { obj.assume_init() };
        assert_ne!(unsafe { obj.deref::<u8>() }, ptr::null());

        let handle = unsafe { mun_gc_root(driver.runtime, obj) };

        assert_eq!(handle.0, ptr::null());

        let mut reclaimed = false;
        let handle = unsafe { mun_gc_collect(driver.runtime, &mut reclaimed as *mut _) };
        assert_eq!(handle.0, ptr::null());
        assert!(!reclaimed);

        let handle = unsafe { mun_gc_unroot(driver.runtime, obj) };
        assert_eq!(handle.0, ptr::null());

        let handle = unsafe { mun_gc_collect(driver.runtime, &mut reclaimed as *mut _) };
        assert_eq!(handle.0, ptr::null());
        assert!(reclaimed);
    }

    #[test]
    fn test_gc_ptr_collect_invalid_reclaimed() {
        let driver = TestDriver::new(
            r#"
        pub struct Foo;

        pub fn main() -> Foo { Foo }
    "#,
        );

        let handle = unsafe { mun_gc_collect(driver.runtime, ptr::null_mut()) };

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'reclaimed' is null pointer."
        );

        unsafe { mun_error_destroy(handle) };
    }
}

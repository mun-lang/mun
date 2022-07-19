//! Exposes Mun garbage collection.

use capi_utils::error::ErrorHandle;
use capi_utils::{mun_error_try, try_deref_mut};
use std::mem::ManuallyDrop;

use crate::runtime::RuntimeHandle;
use memory::ffi::Type;

pub use memory::gc::GcPtr;

/// Allocates an object in the runtime of the given `ty`. If successful, `obj` is set,
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
    ty: Type,
    obj: *mut GcPtr,
) -> ErrorHandle {
    let runtime = mun_error_try!(runtime.inner());
    let ty = ManuallyDrop::new(mun_error_try!(ty.to_owned()));
    let obj = try_deref_mut!(obj);
    *obj = runtime.gc().alloc(&ty);
    ErrorHandle::default()
}

/// Retrieves the `ty` for the specified `obj` from the runtime. If successful, `ty` is set,
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
pub unsafe extern "C" fn mun_gc_ptr_type(
    runtime: RuntimeHandle,
    obj: GcPtr,
    ty: *mut Type,
) -> ErrorHandle {
    let runtime = mun_error_try!(runtime.inner());
    let ty = try_deref_mut!(ty);
    *ty = runtime.gc().ptr_type(obj).into();
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
pub unsafe extern "C" fn mun_gc_root(runtime: RuntimeHandle, obj: GcPtr) -> ErrorHandle {
    let runtime = mun_error_try!(runtime.inner());
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
pub unsafe extern "C" fn mun_gc_unroot(runtime: RuntimeHandle, obj: GcPtr) -> ErrorHandle {
    let runtime = mun_error_try!(runtime.inner());
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
    runtime: RuntimeHandle,
    reclaimed: *mut bool,
) -> ErrorHandle {
    let runtime = mun_error_try!(runtime.inner());
    let reclaimed = try_deref_mut!(reclaimed);
    *reclaimed = runtime.gc_collect();
    ErrorHandle::default()
}

#[cfg(test)]
mod tests {
    use memory::ffi::Type;
    use memory::gc::{HasIndirectionPtr, RawGcPtr};

    use super::*;
    use crate::{
        runtime::mun_runtime_get_type_info_by_name, test_invalid_runtime, test_util::TestDriver,
    };
    use capi_utils::error::mun_error_destroy;
    use std::{
        ffi::{CStr, CString},
        mem::{self, MaybeUninit},
        ptr,
    };

    test_invalid_runtime!(
        gc_alloc(Type::null(), ptr::null_mut()),
        gc_ptr_type(mem::zeroed::<GcPtr>(), ptr::null_mut()),
        gc_root(mem::zeroed::<GcPtr>()),
        gc_unroot(mem::zeroed::<GcPtr>()),
        gc_collect(ptr::null_mut())
    );

    #[test]
    fn test_gc_alloc_invalid_type_info() {
        let driver = TestDriver::new(
            r#"
        pub struct Foo;
    "#,
        );

        let handle = unsafe { mun_gc_alloc(driver.runtime, Type::null(), ptr::null_mut()) };
        assert_ne!(handle.0, ptr::null());

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'type_info' is null pointer."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_gc_alloc_invalid_obj() {
        let driver = TestDriver::new(
            r#"
        pub struct Foo;
    "#,
        );

        let type_name = CString::new("Foo").expect("Invalid type name.");
        let mut has_type = false;
        let mut ty = Type::null();

        let handle = unsafe {
            mun_runtime_get_type_info_by_name(
                driver.runtime,
                type_name.as_ptr(),
                &mut has_type as *mut bool,
                &mut ty as *mut Type,
            )
        };
        assert_eq!(handle.0, ptr::null());

        let handle = unsafe { mun_gc_alloc(driver.runtime, ty, ptr::null_mut()) };
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
        let mut ty = Type::null();

        let handle = unsafe {
            mun_runtime_get_type_info_by_name(
                driver.runtime,
                type_name.as_ptr(),
                &mut has_type as *mut bool,
                &mut ty as *mut Type,
            )
        };
        assert_eq!(handle.0, ptr::null());

        let mut obj = MaybeUninit::uninit();
        let handle = unsafe { mun_gc_alloc(driver.runtime, ty, obj.as_mut_ptr()) };
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
        let mut ty = Type::null();

        let handle = unsafe {
            mun_runtime_get_type_info_by_name(
                driver.runtime,
                type_name.as_ptr(),
                &mut has_type as *mut bool,
                &mut ty as *mut Type,
            )
        };
        assert_eq!(handle.0, ptr::null());

        let mut obj = MaybeUninit::uninit();
        let handle = unsafe { mun_gc_alloc(driver.runtime, ty, obj.as_mut_ptr()) };
        assert_eq!(handle.0, ptr::null());

        let obj = unsafe { obj.assume_init() };
        assert_ne!(unsafe { obj.deref::<u8>() }, ptr::null());

        let mut ty = MaybeUninit::uninit();
        let handle = unsafe { mun_gc_ptr_type(driver.runtime, obj, ty.as_mut_ptr()) };
        assert_eq!(handle.0, ptr::null());

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
        let mut ty = Type::null();

        let handle = unsafe {
            mun_runtime_get_type_info_by_name(
                driver.runtime,
                type_name.as_ptr(),
                &mut has_type as *mut bool,
                &mut ty as *mut Type,
            )
        };
        assert_eq!(handle.0, ptr::null());

        let mut obj = MaybeUninit::uninit();
        let handle = unsafe { mun_gc_alloc(driver.runtime, ty, obj.as_mut_ptr()) };
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

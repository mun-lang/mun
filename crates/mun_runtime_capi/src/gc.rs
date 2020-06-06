//! Exposes Mun garbage collection.

use crate::{ErrorHandle, RuntimeHandle, HUB};
use anyhow::anyhow;
use runtime::Runtime;

pub use memory::gc::GcPtr;
pub use runtime::UnsafeTypeInfo;

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
    handle: RuntimeHandle,
    type_info: UnsafeTypeInfo,
    obj: *mut GcPtr,
) -> ErrorHandle {
    let runtime = match (handle.0 as *mut Runtime).as_ref() {
        Some(runtime) => runtime,
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'runtime' is null pointer."))
        }
    };

    let obj = match obj.as_mut() {
        Some(obj) => obj,
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'obj' is null pointer."))
        }
    };

    *obj = runtime.gc().alloc(type_info);
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
    type_info: *mut UnsafeTypeInfo,
) -> ErrorHandle {
    let runtime = match (handle.0 as *mut Runtime).as_ref() {
        Some(runtime) => runtime,
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'runtime' is null pointer."))
        }
    };

    let type_info = match type_info.as_mut() {
        Some(type_info) => type_info,
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'type_info' is null pointer."))
        }
    };

    *type_info = runtime.gc().ptr_type(obj);
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
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'runtime' is null pointer."))
        }
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
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'runtime' is null pointer."))
        }
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
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'runtime' is null pointer."))
        }
    };

    let reclaimed = match reclaimed.as_mut() {
        Some(reclaimed) => reclaimed,
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'reclaimed' is null pointer."))
        }
    };

    *reclaimed = runtime.gc_collect();
    ErrorHandle::default()
}

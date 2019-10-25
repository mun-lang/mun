use mun_abi::FunctionInfo;
use mun_runtime::{MunRuntime, RuntimeBuilder};
use std::ffi::{c_void, CStr};
use std::os::raw::c_char;

#[repr(C)]
pub struct RuntimeHandle(*mut c_void);

#[no_mangle]
pub extern "C" fn create_runtime(library_path: *const c_char, handle: *mut RuntimeHandle) -> u64 /* error */
{
    if library_path.is_null() {
        return 1;
    }

    let library_path = match unsafe { CStr::from_ptr(library_path) }.to_str() {
        Ok(path) => path,
        Err(_) => return 2,
    };

    let handle = match unsafe { handle.as_mut() } {
        Some(handle) => handle,
        None => return 3,
    };

    let runtime = match RuntimeBuilder::new(library_path).spawn() {
        Ok(runtime) => runtime,
        Err(_) => return 4,
    };

    handle.0 = Box::into_raw(Box::new(runtime)) as *mut _;
    0
}

#[no_mangle]
pub extern "C" fn destroy_runtime(handle: RuntimeHandle) {
    if !handle.0.is_null() {
        let _runtime = unsafe { Box::from_raw(handle.0) };
    }
}

#[no_mangle]
pub extern "C" fn runtime_get_function_info(
    handle: RuntimeHandle,
    fn_name: *const c_char,
    has_fn_info: *mut bool,
    fn_info: *mut FunctionInfo,
) -> u64 /* error */ {
    let runtime = match unsafe { (handle.0 as *mut MunRuntime).as_ref() } {
        Some(runtime) => runtime,
        None => return 1,
    };

    let fn_name = match unsafe { CStr::from_ptr(fn_name) }.to_str() {
        Ok(name) => name,
        Err(_) => return 2,
    };

    let has_fn_info = match unsafe { has_fn_info.as_mut() } {
        Some(has_info) => has_info,
        None => return 3,
    };

    let fn_info = match unsafe { fn_info.as_mut() } {
        Some(info) => info,
        None => return 4,
    };

    match runtime.get_function_info(fn_name) {
        Some(info) => {
            *has_fn_info = true;
            *fn_info = info.clone();
        }
        None => *has_fn_info = false,
    }

    0
}

#[no_mangle]
pub extern "C" fn runtime_update(handle: RuntimeHandle, updated: *mut bool) -> u64 /* error */ {
    let runtime = match unsafe { (handle.0 as *mut MunRuntime).as_mut() } {
        Some(runtime) => runtime,
        None => return 1,
    };

    let updated = match unsafe { updated.as_mut() } {
        Some(updated) => updated,
        None => return 2,
    };

    *updated = runtime.update();
    0
}

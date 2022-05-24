use crate::{error::ErrorHandle, hub::HUB, type_info::TypeInfoHandle};
use anyhow::anyhow;
use memory::FieldInfo;
use std::{
    ffi::{c_void, CString},
    os::raw::c_char,
    ptr,
    sync::Arc,
};

/// A C-style handle to a `FieldInfo`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FieldInfoHandle(pub *const c_void);

/// Retrieves the field's name.
///
/// # Safety
///
/// The caller is responsible for calling `mun_destroy_string` on the return pointer - if it is not null.
#[no_mangle]
pub unsafe extern "C" fn mun_field_info_name(field_info: FieldInfoHandle) -> *const c_char {
    let field_info = match (field_info.0 as *const FieldInfo).as_ref() {
        Some(field_info) => field_info,
        None => return ptr::null(),
    };

    CString::new(field_info.name.clone()).unwrap().into_raw() as *const _
}

/// Retrieves the field's type.
#[no_mangle]
pub unsafe extern "C" fn mun_field_info_type(field_info: FieldInfoHandle) -> TypeInfoHandle {
    let field_info = match (field_info.0 as *const FieldInfo).as_ref() {
        Some(field_info) => field_info,
        None => return TypeInfoHandle::null(),
    };

    TypeInfoHandle(Arc::into_raw(field_info.type_info.clone()) as *const c_void)
}

/// Retrieves the field's offset.
#[no_mangle]
pub unsafe extern "C" fn mun_field_info_offset(
    field_info: FieldInfoHandle,
    field_offset: *mut u16,
) -> ErrorHandle {
    let field_info = match (field_info.0 as *const FieldInfo).as_ref() {
        Some(field_info) => field_info,
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'field_info' is null pointer."))
        }
    };

    let field_offset = match field_offset.as_mut() {
        Some(field_offset) => field_offset,
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'field_offset' is null pointer."))
        }
    };

    *field_offset = field_info.offset;

    ErrorHandle::default()
}

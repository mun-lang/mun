use crate::{error::ErrorHandle, hub::HUB, struct_info::StructInfoHandle};
use anyhow::anyhow;
use memory::{StructInfo, TypeInfo};
use std::{
    ffi::{c_void, CString},
    os::raw::c_char,
    ptr,
    sync::Arc,
};

/// A C-style handle to a `TypeInfo`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TypeInfoHandle(pub *const c_void);

impl TypeInfoHandle {
    pub fn null() -> Self {
        Self(ptr::null())
    }
}

/// Decrements the strong count of the `Arc<TypeInfo>` associated with `handle`.
#[no_mangle]
pub unsafe extern "C" fn mun_type_info_decrement_strong_count(handle: TypeInfoHandle) {
    if !handle.0.is_null() {
        Arc::decrement_strong_count(handle.0);
    }
}

/// Increments the strong count of the `Arc<TypeInfo>` associated with `handle`.
#[no_mangle]
pub unsafe extern "C" fn mun_type_info_increment_strong_count(handle: TypeInfoHandle) {
    if !handle.0.is_null() {
        Arc::increment_strong_count(handle.0);
    }
}

/// Retrieves the type's ID.
#[no_mangle]
pub unsafe extern "C" fn mun_type_info_id(
    type_info: TypeInfoHandle,
    type_id: *mut abi::TypeId,
) -> ErrorHandle {
    let type_info = match (type_info.0 as *const TypeInfo).as_ref() {
        Some(type_info) => type_info,
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'handle' is null pointer."))
        }
    };

    let type_id = match type_id.as_mut() {
        Some(type_id) => type_id,
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'type_id' is null pointer."))
        }
    };

    *type_id = type_info.id.clone();

    ErrorHandle::default()
}

/// Retrieves the type's name.
///
/// # Safety
///
/// The caller is responsible for calling `mun_destroy_string` on the return pointer - if it is not null.
#[no_mangle]
pub unsafe extern "C" fn mun_type_info_name(type_info: TypeInfoHandle) -> *const c_char {
    let type_info = match (type_info.0 as *const TypeInfo).as_ref() {
        Some(type_info) => type_info,
        None => return ptr::null(),
    };

    CString::new(type_info.name.clone()).unwrap().into_raw() as *const _
}

/// Retrieves the type's size.
#[no_mangle]
pub unsafe extern "C" fn mun_type_info_size(
    type_info: TypeInfoHandle,
    size: *mut usize,
) -> ErrorHandle {
    let type_info = match (type_info.0 as *const TypeInfo).as_ref() {
        Some(type_info) => type_info,
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'type_info' is null pointer."))
        }
    };

    let size = match size.as_mut() {
        Some(size) => size,
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'size' is null pointer."))
        }
    };

    *size = type_info.layout.size();

    ErrorHandle::default()
}

/// Retrieves the type's alignment.
#[no_mangle]
pub unsafe extern "C" fn mun_type_info_align(
    type_info: TypeInfoHandle,
    align: *mut usize,
) -> ErrorHandle {
    let type_info = match (type_info.0 as *const TypeInfo).as_ref() {
        Some(type_info) => type_info,
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'type_info' is null pointer."))
        }
    };

    let align = match align.as_mut() {
        Some(align) => align,
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'align' is null pointer."))
        }
    };

    *align = type_info.layout.align();

    ErrorHandle::default()
}

/// An enum containing C-style handles a `TypeInfo`'s data.
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum TypeInfoData {
    /// Primitive types (i.e. `()`, `bool`, `float`, `int`, etc.)
    Primitive,
    /// Struct types (i.e. record, tuple, or unit structs)
    Struct(StructInfoHandle),
}

/// Retrieves the type's data.
#[no_mangle]
pub unsafe extern "C" fn mun_type_info_data(
    type_info: TypeInfoHandle,
    type_info_data: *mut TypeInfoData,
) -> ErrorHandle {
    let type_info = match (type_info.0 as *const TypeInfo).as_ref() {
        Some(type_info) => type_info,
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'type_info' is null pointer."))
        }
    };

    let type_info_data = match type_info_data.as_mut() {
        Some(type_info_data) => type_info_data,
        None => {
            return HUB.errors.register(anyhow!(
                "Invalid argument: 'type_info_data' is null pointer."
            ))
        }
    };

    *type_info_data = match &type_info.data {
        memory::TypeInfoData::Primitive => TypeInfoData::Primitive,
        memory::TypeInfoData::Struct(s) => {
            TypeInfoData::Struct(StructInfoHandle(s as *const StructInfo as *const c_void))
        }
    };

    ErrorHandle::default()
}

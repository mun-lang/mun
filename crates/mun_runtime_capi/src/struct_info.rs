use crate::{error::ErrorHandle, field_info::FieldInfoHandle, hub::HUB};
use anyhow::anyhow;
use memory::StructInfo;
use std::ffi::c_void;

/// A C-style handle to a `StructInfo`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct StructInfoHandle(pub *const c_void);

/// Retrieves the struct's fields.
///
/// If `struct_handle` is null, the returned will
#[no_mangle]
pub unsafe extern "C" fn mun_struct_info_fields(
    struct_info: StructInfoHandle,
    field_infos_begin: *mut FieldInfoHandle,
    num_fields: *mut usize,
) -> ErrorHandle {
    let struct_info = match (struct_info.0 as *const StructInfo).as_ref() {
        Some(struct_info) => struct_info,
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'struct_info' is null pointer."))
        }
    };

    let field_infos_begin = match field_infos_begin.as_mut() {
        Some(field_infos_begin) => field_infos_begin,
        None => {
            return HUB.errors.register(anyhow!(
                "Invalid argument: 'field_infos_begin' is null pointer."
            ))
        }
    };

    let num_fields = match num_fields.as_mut() {
        Some(num_fields) => num_fields,
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'num_fields' is null pointer."))
        }
    };

    field_infos_begin.0 = struct_info.fields.as_ptr() as *const c_void;
    *num_fields = struct_info.fields.len();

    ErrorHandle::default()
}

/// Retrieves the struct's memory kind.
#[no_mangle]
pub unsafe extern "C" fn mun_struct_info_memory_kind(
    struct_info: StructInfoHandle,
    memory_kind: *mut abi::StructMemoryKind,
) -> ErrorHandle {
    let struct_info = match (struct_info.0 as *const StructInfo).as_ref() {
        Some(struct_info) => struct_info,
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'struct_info' is null pointer."))
        }
    };

    let memory_kind = match memory_kind.as_mut() {
        Some(memory_kind) => memory_kind,
        None => {
            return HUB
                .errors
                .register(anyhow!("Invalid argument: 'memory_kind' is null pointer."))
        }
    };

    *memory_kind = struct_info.memory_kind;

    ErrorHandle::default()
}

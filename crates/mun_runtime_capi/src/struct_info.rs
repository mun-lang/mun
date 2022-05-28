//! Exposes struct information using the C ABI.

use crate::{
    error::ErrorHandle,
    field_info::{FieldInfoHandle, FieldInfoSpan},
};
use memory::{FieldInfo, StructInfo};
use std::{ffi::c_void, mem, ptr};

/// A C-style handle to a `StructInfo`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct StructInfoHandle(pub *const c_void);

/// Retrieves information about the struct's fields.
///
/// # Safety
///
/// The caller is responsible for calling `mun_field_info_span_destroy` on the returned span.
#[no_mangle]
pub unsafe extern "C" fn mun_struct_info_fields(
    struct_info: StructInfoHandle,
    field_info_span: *mut FieldInfoSpan,
) -> ErrorHandle {
    let struct_info = match (struct_info.0 as *const StructInfo).as_ref() {
        Some(struct_info) => struct_info,
        None => return ErrorHandle::new("Invalid argument: 'struct_info' is null pointer."),
    };

    let field_info_span = match field_info_span.as_mut() {
        Some(field_info_span) => field_info_span,
        None => return ErrorHandle::new("Invalid argument: 'field_info_span' is null pointer."),
    };

    let mut fields: Vec<FieldInfoHandle> = struct_info
        .fields
        .iter()
        .map(|field| FieldInfoHandle(field as *const FieldInfo as *const c_void))
        .collect();

    field_info_span.len = fields.len();
    field_info_span.data = if fields.is_empty() {
        ptr::null()
    } else {
        fields.shrink_to_fit();
        fields.as_ptr() as *const _
    };

    // Ownership is transferred
    mem::forget(fields);

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
        None => return ErrorHandle::new("Invalid argument: 'struct_info' is null pointer."),
    };

    let memory_kind = match memory_kind.as_mut() {
        Some(memory_kind) => memory_kind,
        None => return ErrorHandle::new("Invalid argument: 'memory_kind' is null pointer."),
    };

    *memory_kind = struct_info.memory_kind;

    ErrorHandle::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        error::mun_error_destroy,
        field_info::{mun_field_info_name, mun_field_info_span_destroy},
        runtime::RuntimeHandle,
        test_util::TestDriver,
        type_info::{mun_type_info_data, tests::get_type_info_by_name},
    };
    use std::{ffi::CStr, mem::MaybeUninit, ptr, slice};

    fn get_struct_info_by_name<T: Into<Vec<u8>>>(
        runtime: RuntimeHandle,
        type_name: T,
    ) -> StructInfoHandle {
        let type_info = get_type_info_by_name(runtime, type_name);

        let mut type_info_data = MaybeUninit::uninit();
        let handle = unsafe { mun_type_info_data(type_info, type_info_data.as_mut_ptr()) };
        assert_eq!(handle.0, ptr::null());

        let type_info_data = unsafe { type_info_data.assume_init() };
        type_info_data
            .as_struct()
            .expect("the type name does not represent a struct")
    }

    #[test]
    fn test_mun_struct_info_fields_invalid_arg() {
        let driver = TestDriver::new(
            r#"
            pub struct Foo;"#,
        );

        let foo_struct_info = get_struct_info_by_name(driver.runtime, "Foo");
        let handle = unsafe { mun_struct_info_fields(foo_struct_info, ptr::null_mut()) };
        assert_ne!(handle.0, ptr::null());

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'field_info_span' is null pointer."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_mun_struct_info_fields() {
        let driver = TestDriver::new(
            r#"
            pub struct Foo {
                bar: i8,
                baz: f32,
            }"#,
        );

        let foo_struct_info = get_struct_info_by_name(driver.runtime, "Foo");

        let mut foo_fields_span = MaybeUninit::uninit();
        let handle =
            unsafe { mun_struct_info_fields(foo_struct_info, foo_fields_span.as_mut_ptr()) };
        assert_eq!(handle.0, ptr::null());

        let foo_fields_span = unsafe { foo_fields_span.assume_init() };
        assert_eq!(foo_fields_span.len, 2);

        let foo_fields: Vec<_> =
            unsafe { slice::from_raw_parts(foo_fields_span.data, foo_fields_span.len) }
                .iter()
                .map(|field_info| {
                    unsafe { CStr::from_ptr(mun_field_info_name(*field_info)) }
                        .to_str()
                        .expect("invalid field name")
                        .to_owned()
                })
                .collect();

        assert!(unsafe { mun_field_info_span_destroy(foo_fields_span) });

        assert_eq!(vec![String::from("bar"), String::from("baz")], foo_fields);
    }

    #[test]
    fn test_mun_struct_info_memory_kind() {
        let driver = TestDriver::new(
            r#"
            pub struct Foo {
                bar: i8,
                baz: f32,
            }

            pub struct(value) Bar;"#,
        );

        let foo_struct_info = get_struct_info_by_name(driver.runtime, "Foo");
        let bar_struct_info = get_struct_info_by_name(driver.runtime, "Bar");

        let mut foo_memory_kind = MaybeUninit::uninit();
        let handle =
            unsafe { mun_struct_info_memory_kind(foo_struct_info, foo_memory_kind.as_mut_ptr()) };
        assert_eq!(handle.0, ptr::null());

        let mut bar_memory_kind = MaybeUninit::uninit();
        let handle =
            unsafe { mun_struct_info_memory_kind(bar_struct_info, bar_memory_kind.as_mut_ptr()) };
        assert_eq!(handle.0, ptr::null());

        let foo_memory_kind = unsafe { foo_memory_kind.assume_init() };
        let bar_memory_kind = unsafe { bar_memory_kind.assume_init() };

        assert_eq!(foo_memory_kind, abi::StructMemoryKind::Gc);
        assert_eq!(bar_memory_kind, abi::StructMemoryKind::Value);
    }
}

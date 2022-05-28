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

impl FieldInfoHandle {
    /// A null handle.
    pub fn null() -> Self {
        FieldInfoHandle(ptr::null())
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        error::mun_error_message,
        handle::TypedHandle,
        mun_destroy_string,
        runtime::{mun_runtime_get_type_info_by_name, RuntimeHandle},
        struct_info::mun_struct_info_fields,
        test_util::TestDriver,
        type_info::{mun_type_info_data, mun_type_info_id},
    };
    use memory::HasStaticTypeInfo;
    use std::{
        ffi::{CStr, CString},
        mem::MaybeUninit,
    };

    fn get_first_field<T: Into<Vec<u8>>>(runtime: RuntimeHandle, type_name: T) -> FieldInfoHandle {
        let type_name = CString::new(type_name).expect("Invalid type name");
        let mut has_type_info = false;
        let mut type_info = MaybeUninit::uninit();
        let handle = unsafe {
            mun_runtime_get_type_info_by_name(
                runtime,
                type_name.as_ptr(),
                &mut has_type_info as *mut bool,
                type_info.as_mut_ptr(),
            )
        };
        assert_eq!(handle.token(), 0);
        assert!(has_type_info);

        let type_info = unsafe { type_info.assume_init() };

        let mut type_info_data = MaybeUninit::uninit();
        let handle = unsafe { mun_type_info_data(type_info, type_info_data.as_mut_ptr()) };
        assert_eq!(handle.token(), 0);

        let type_info_data = unsafe { type_info_data.assume_init() };
        let struct_info = type_info_data
            .as_struct()
            .expect("Type was expected to be a struct.");

        let mut field_info = MaybeUninit::uninit();
        let mut num_fields = MaybeUninit::uninit();
        let handle = unsafe {
            mun_struct_info_fields(
                struct_info,
                field_info.as_mut_ptr(),
                num_fields.as_mut_ptr(),
            )
        };
        assert_eq!(handle.token(), 0);

        let num_fields = unsafe { num_fields.assume_init() };
        assert!(num_fields > 0);

        unsafe { field_info.assume_init() }
    }

    #[test]
    fn test_field_info_name_invalid_handle() {
        let name = unsafe { mun_field_info_name(FieldInfoHandle::null()) };
        assert_eq!(name, ptr::null());
    }

    #[test]
    fn test_field_info_name() {
        let driver = TestDriver::new(
            r#"
            pub struct Foo {
                a: i32,
            }
    "#,
        );

        let field_info = get_first_field(driver.runtime, "Foo");
        let name = unsafe { mun_field_info_name(field_info) };
        assert_ne!(name, ptr::null());

        let name = unsafe { CStr::from_ptr(name) }
            .to_str()
            .expect("Invalid field name.");

        assert_eq!(name, "a");
    }
    #[test]
    fn test_field_info_type_invalid_handle() {
        let type_info = unsafe { mun_field_info_type(FieldInfoHandle::null()) };
        assert_eq!(type_info.0, ptr::null());
    }

    #[test]
    fn test_field_info_type() {
        let driver = TestDriver::new(
            r#"
            pub struct Foo {
                a: i32,
            }
    "#,
        );

        let field_info = get_first_field(driver.runtime, "Foo");
        let type_info = unsafe { mun_field_info_type(field_info) };
        assert_ne!(type_info.0, ptr::null());

        let mut type_id = MaybeUninit::uninit();
        let handle = unsafe { mun_type_info_id(type_info, type_id.as_mut_ptr()) };
        assert_eq!(handle.token(), 0);

        let type_id = unsafe { type_id.assume_init() };
        assert_eq!(type_id, <i32>::type_info().id);
    }

    #[test]
    fn test_field_info_offset_invalid_field_info() {
        let mut field_offset = MaybeUninit::uninit();
        let handle =
            unsafe { mun_field_info_offset(FieldInfoHandle::null(), field_offset.as_mut_ptr()) };
        assert_ne!(handle.token(), 0);

        let message = unsafe { CStr::from_ptr(mun_error_message(handle)) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'field_info' is null pointer."
        );

        unsafe { mun_destroy_string(message.as_ptr()) };
    }

    #[test]
    fn test_field_info_offset_invalid_field_offset() {
        let driver = TestDriver::new(
            r#"
            pub struct Foo {
                a: i32,
            }
    "#,
        );

        let field_info = get_first_field(driver.runtime, "Foo");

        let handle = unsafe { mun_field_info_offset(field_info, ptr::null_mut()) };
        assert_ne!(handle.token(), 0);

        let message = unsafe { CStr::from_ptr(mun_error_message(handle)) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'field_offset' is null pointer."
        );

        unsafe { mun_destroy_string(message.as_ptr()) };
    }

    #[test]
    fn test_field_info_offset() {
        let driver = TestDriver::new(
            r#"
            pub struct Foo {
                a: i32,
            }
    "#,
        );

        let field_info = get_first_field(driver.runtime, "Foo");

        let mut field_offset = MaybeUninit::uninit();
        let handle = unsafe { mun_field_info_offset(field_info, field_offset.as_mut_ptr()) };
        assert_eq!(handle.token(), 0);

        let field_offset = unsafe { field_offset.assume_init() };
        assert_eq!(field_offset, 0);
    }
}

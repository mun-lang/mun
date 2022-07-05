//! Exposes type information using the C ABI.

use crate::{error::ErrorHandle, struct_info::StructInfoHandle};
use memory::{HasStaticTypeInfo, StructInfo, TypeInfo};
use std::mem::ManuallyDrop;
use std::ops::Deref;
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
    /// A null handle.
    pub fn null() -> Self {
        Self(ptr::null())
    }
}

impl From<Arc<TypeInfo>> for TypeInfoHandle {
    fn from(ty: Arc<TypeInfo>) -> Self {
        TypeInfoHandle(Arc::into_raw(ty) as _)
    }
}

/// A C-style handle to an array of `TypeInfoHandle`s.
#[repr(C)]
pub struct TypeInfoSpan {
    /// Pointer to the start of the array buffer
    pub data: *const TypeInfoHandle,
    /// Length of the array (and capacity)
    pub len: usize,
}

impl TypeInfoSpan {
    /// Retrieves the `TypeInfoHandle` at the specified index, if within bounds.
    /// Otherwise, returns null.
    pub fn get(&self, index: usize) -> TypeInfoHandle {
        if index < self.len {
            // SAFETY: Bounds checking was performed
            TypeInfoHandle(unsafe {
                *(self.data as *const *const TypeInfo).add(index) as *const c_void
            })
        } else {
            TypeInfoHandle::null()
        }
    }
}

/// Decrements the strong count of the `Arc<TypeInfo>` associated with `handle`.
///
/// # Safety
///
/// This function results in undefined behavior if the passed in `TypeInfoHandle` has already been
/// deallocated in a previous call to [`mun_type_info_decrement_strong_count`].
#[no_mangle]
pub unsafe extern "C" fn mun_type_info_decrement_strong_count(handle: TypeInfoHandle) -> bool {
    if !handle.0.is_null() {
        Arc::decrement_strong_count(handle.0);
        return true;
    }

    false
}

/// Increments the strong count of the `Arc<TypeInfo>` associated with `handle`.
///
/// # Safety
///
/// This function results in undefined behavior if the passed in `TypeInfoHandle` has already been
/// deallocated in a previous call to [`mun_type_info_decrement_strong_count`].
#[no_mangle]
pub unsafe extern "C" fn mun_type_info_increment_strong_count(handle: TypeInfoHandle) -> bool {
    if !handle.0.is_null() {
        Arc::increment_strong_count(handle.0);
        return true;
    }

    false
}

/// Retrieves the type's name.
///
/// # Safety
///
/// The caller is responsible for calling `mun_string_destroy` on the return pointer - if it is not null.
///
/// This function results in undefined behavior if the passed in `TypeInfoHandle` has been
/// deallocated in a previous call to [`mun_type_info_decrement_strong_count`].
#[no_mangle]
pub unsafe extern "C" fn mun_type_info_name(type_info: TypeInfoHandle) -> *const c_char {
    let type_info = match (type_info.0 as *const TypeInfo).as_ref() {
        Some(type_info) => type_info,
        None => return ptr::null(),
    };

    CString::new(type_info.name.clone()).unwrap().into_raw() as *const _
}

/// Returns true if the specified type info handles describe the same type.
///
/// # Safety
///
/// This function results in undefined behavior if any of the the passed in `TypeInfoHandle` have
/// been deallocated in a previous call to [`mun_type_info_decrement_strong_count`].
#[no_mangle]
pub unsafe extern "C" fn mun_type_info_eq(a: TypeInfoHandle, b: TypeInfoHandle) -> bool {
    (a.0 as *const TypeInfo).as_ref() == (b.0 as *const TypeInfo).as_ref()
}

/// Returns the TypeInfoHandle of a pointer to the given TypeInfoHandle.
///
/// # Safety
///
/// This function results in undefined behavior if any of the the passed in `TypeInfoHandle` have
/// been deallocated in a previous call to [`mun_type_info_decrement_strong_count`].
#[no_mangle]
pub unsafe extern "C" fn mun_type_info_pointer_type(
    handle: TypeInfoHandle,
    mutable: bool,
) -> TypeInfoHandle {
    if handle.0.is_null() {
        return TypeInfoHandle::null();
    }

    let type_info = ManuallyDrop::new(Arc::from_raw(handle.0 as *const TypeInfo));
    type_info.deref().pointer_type(mutable).into()
}

/// Retrieves the type's size.
///
/// # Safety
///
/// This function results in undefined behavior if the passed in `TypeInfoHandle` has been
/// deallocated in a previous call to [`mun_type_info_decrement_strong_count`].
#[no_mangle]
pub unsafe extern "C" fn mun_type_info_size(
    type_info: TypeInfoHandle,
    size: *mut usize,
) -> ErrorHandle {
    let type_info = match (type_info.0 as *const TypeInfo).as_ref() {
        Some(type_info) => type_info,
        None => return ErrorHandle::new("Invalid argument: 'type_info' is null pointer."),
    };

    let size = match size.as_mut() {
        Some(size) => size,
        None => return ErrorHandle::new("Invalid argument: 'size' is null pointer."),
    };

    *size = type_info.layout.size();

    ErrorHandle::default()
}

/// Retrieves the type's alignment.
///
/// # Safety
///
/// This function results in undefined behavior if the passed in `TypeInfoHandle` has been
/// deallocated in a previous call to [`mun_type_info_decrement_strong_count`].
#[no_mangle]
pub unsafe extern "C" fn mun_type_info_align(
    type_info: TypeInfoHandle,
    align: *mut usize,
) -> ErrorHandle {
    let type_info = match (type_info.0 as *const TypeInfo).as_ref() {
        Some(type_info) => type_info,
        None => return ErrorHandle::new("Invalid argument: 'type_info' is null pointer."),
    };

    let align = match align.as_mut() {
        Some(align) => align,
        None => return ErrorHandle::new("Invalid argument: 'align' is null pointer."),
    };

    *align = type_info.layout.align();

    ErrorHandle::default()
}

/// An enum containing C-style handles a `TypeInfo`'s data.
/// cbindgen:prefix-with-name=true
#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum TypeInfoData {
    /// Primitive types (i.e. `()`, `bool`, `float`, `int`, etc.)
    Primitive(abi::Guid),
    /// Struct types (i.e. record, tuple, or unit structs)
    Struct(StructInfoHandle),
    /// A pointer type
    Pointer(PointerInfoData),
}

/// A pointer to another type.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PointerInfoData {
    //pointee: TypeInfoHandle,
    mutable: bool,
}

impl TypeInfoData {
    /// Whether the type is a primitive.
    #[cfg(test)]
    fn is_primitive(&self) -> bool {
        matches!(*self, TypeInfoData::Primitive(_))
    }

    /// Whether the type is a struct.
    #[cfg(test)]
    fn is_struct(&self) -> bool {
        matches!(*self, TypeInfoData::Struct(_))
    }

    /// Returns the C-style handle to the struct information, if available.
    #[cfg(test)]
    pub(crate) fn as_struct(&self) -> Option<StructInfoHandle> {
        if let TypeInfoData::Struct(handle) = self {
            Some(*handle)
        } else {
            None
        }
    }
}

/// Retrieves the type's data.
///
/// # Safety
///
/// The original `TypeInfoHandle` needs to stay alive as long as the `TypeInfoData` lives. The
/// `TypeInfoData` is destroyed at the same time as the `TypeInfo`. A `TypeInfo` might be destroyed
/// through a call to [`mun_type_info_decrement_strong_count`].
#[no_mangle]
pub unsafe extern "C" fn mun_type_info_data(
    type_info: TypeInfoHandle,
    type_info_data: *mut TypeInfoData,
) -> ErrorHandle {
    let type_info = match (type_info.0 as *const TypeInfo).as_ref() {
        Some(type_info) => type_info,
        None => return ErrorHandle::new("Invalid argument: 'type_info' is null pointer."),
    };

    let type_info_data = match type_info_data.as_mut() {
        Some(type_info_data) => type_info_data,
        None => return ErrorHandle::new("Invalid argument: 'type_info_data' is null pointer."),
    };

    *type_info_data = match &type_info.data {
        memory::TypeInfoData::Primitive(guid) => TypeInfoData::Primitive(*guid),
        memory::TypeInfoData::Struct(s) => {
            TypeInfoData::Struct(StructInfoHandle(s as *const StructInfo as *const c_void))
        }
        memory::TypeInfoData::Pointer(pointer) => {
            TypeInfoData::Pointer(PointerInfoData {
                //pointee: TypeInfoHandle(Arc::into_raw(pointer.pointee.clone()) as _),
                mutable: pointer.mutable,
            })
        }
    };

    ErrorHandle::default()
}

/// Deallocates an span of `TypeInfo` that was allocated by the runtime.
///
/// # Safety
///
/// This function receives a span as parameter. Only when the spans data pointer is not null, its
/// content will be deallocated. Passing pointers to invalid data or memory allocated by other
/// processes, will lead to undefined behavior.
#[no_mangle]
pub unsafe extern "C" fn mun_type_info_span_destroy(array_handle: TypeInfoSpan) -> bool {
    if array_handle.data.is_null() {
        return false;
    }

    let data = array_handle.data as *mut *const TypeInfo;
    let types = Vec::from_raw_parts(data, array_handle.len, array_handle.len);

    types.into_iter().for_each(|ty| {
        let _drop = Arc::from_raw(ty);
    });

    true
}

/// Types of primitives supported by Mun.
/// cbindgen:prefix-with-name=true
#[repr(u8)]
#[derive(Clone, Copy)]
#[allow(missing_docs)]
pub enum PrimitiveType {
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    F32,
    F64,
    Empty,
    Void,
}

/// Returns a TypeInfoHandle that represents the specified primitive type.
#[no_mangle]
pub extern "C" fn mun_type_info_primitive(primitive_type: PrimitiveType) -> TypeInfoHandle {
    match primitive_type {
        PrimitiveType::Bool => bool::type_info(),
        PrimitiveType::U8 => u8::type_info(),
        PrimitiveType::U16 => u16::type_info(),
        PrimitiveType::U32 => u32::type_info(),
        PrimitiveType::U64 => u64::type_info(),
        PrimitiveType::U128 => u128::type_info(),
        PrimitiveType::I8 => i8::type_info(),
        PrimitiveType::I16 => i16::type_info(),
        PrimitiveType::I32 => i32::type_info(),
        PrimitiveType::I64 => i64::type_info(),
        PrimitiveType::I128 => i128::type_info(),
        PrimitiveType::F32 => f32::type_info(),
        PrimitiveType::F64 => f64::type_info(),
        PrimitiveType::Empty => <()>::type_info(),
        PrimitiveType::Void => <std::ffi::c_void>::type_info(),
    }
    .clone()
    .into()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::function_info::mun_function_info_argument_types;
    use crate::function_info::tests::get_fake_function_info;
    use crate::{
        error::mun_error_destroy,
        runtime::{mun_runtime_get_type_info_by_name, RuntimeHandle},
        test_util::TestDriver,
    };
    use std::{
        ffi::{CStr, CString},
        mem::{self, MaybeUninit},
        sync::Arc,
    };

    pub(crate) fn get_type_info_by_name<T: Into<Vec<u8>>>(
        runtime: RuntimeHandle,
        type_name: T,
    ) -> TypeInfoHandle {
        let type_name = CString::new(type_name).expect("Invalid type name");
        let mut has_type_info = MaybeUninit::uninit();
        let mut type_info = MaybeUninit::uninit();
        let handle = unsafe {
            mun_runtime_get_type_info_by_name(
                runtime,
                type_name.as_ptr(),
                has_type_info.as_mut_ptr(),
                type_info.as_mut_ptr(),
            )
        };
        assert_eq!(handle.0, ptr::null());

        let has_type_info = unsafe { has_type_info.assume_init() };
        assert!(has_type_info);

        unsafe { type_info.assume_init() }
    }

    #[test]
    fn test_type_info_decrement_strong_count_invalid_type_info() {
        assert!(!unsafe { mun_type_info_decrement_strong_count(TypeInfoHandle::null()) });
    }

    #[test]
    fn test_type_info_decrement_strong_count() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 12345 }
    "#,
        );

        let type_info = get_type_info_by_name(driver.runtime, "core::i32");

        let type_info_arc = unsafe { Arc::from_raw(type_info.0 as *const TypeInfo) };
        let strong_count = Arc::strong_count(&type_info_arc);
        assert!(strong_count > 0);

        assert!(unsafe { mun_type_info_decrement_strong_count(type_info) });
        assert_eq!(Arc::strong_count(&type_info_arc), strong_count - 1);

        mem::forget(type_info_arc);
    }

    #[test]
    fn test_type_info_increment_strong_count_invalid_type_info() {
        assert_eq!(
            unsafe { mun_type_info_increment_strong_count(TypeInfoHandle::null()) },
            false
        );
    }

    #[test]
    fn test_type_info_increment_strong_count() {
        let driver = TestDriver::new(
            r#"
        struct Foo;
    "#,
        );

        let type_info = get_type_info_by_name(driver.runtime, "Foo");

        let type_info_arc = unsafe { Arc::from_raw(type_info.0 as *const TypeInfo) };
        let strong_count = Arc::strong_count(&type_info_arc);
        assert!(strong_count > 0);

        assert!(unsafe { mun_type_info_increment_strong_count(type_info) },);
        assert_eq!(Arc::strong_count(&type_info_arc), strong_count + 1);

        mem::forget(type_info_arc);
    }

    #[test]
    fn test_type_info_name_invalid_type_info() {
        let fn_ptr = unsafe { mun_type_info_name(TypeInfoHandle::null()) };
        assert_eq!(fn_ptr, ptr::null());
    }

    #[test]
    fn test_type_info_name() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 12345 }
    "#,
        );

        let type_info = get_type_info_by_name(driver.runtime, "core::i32");
        let name = unsafe { mun_type_info_name(type_info) };
        assert_ne!(name, ptr::null());

        let name = unsafe { CStr::from_ptr(name) }
            .to_str()
            .expect("Invalid type name.");

        assert_eq!(name, "core::i32");
    }

    #[test]
    fn test_type_info_size_invalid_type_info() {
        let handle = unsafe { mun_type_info_size(TypeInfoHandle::null(), ptr::null_mut()) };
        assert_ne!(handle.0, ptr::null());

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'type_info' is null pointer."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_type_info_size_invalid_size() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 12345 }
    "#,
        );

        let type_info = get_type_info_by_name(driver.runtime, "core::i32");
        let handle = unsafe { mun_type_info_size(type_info, ptr::null_mut()) };
        assert_ne!(handle.0, ptr::null());

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'size' is null pointer."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_type_info_size() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 12345 }
    "#,
        );

        let type_info = get_type_info_by_name(driver.runtime, "core::i32");
        let mut size = MaybeUninit::uninit();
        let handle = unsafe { mun_type_info_size(type_info, size.as_mut_ptr()) };
        assert_eq!(handle.0, ptr::null());

        let size = unsafe { size.assume_init() };
        assert_eq!(size, mem::size_of::<i32>());
    }

    #[test]
    fn test_type_info_align_invalid_type_info() {
        let handle = unsafe { mun_type_info_align(TypeInfoHandle::null(), ptr::null_mut()) };
        assert_ne!(handle.0, ptr::null());

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'type_info' is null pointer."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_type_info_align_invalid_align() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 12345 }
    "#,
        );

        let type_info = get_type_info_by_name(driver.runtime, "core::i32");
        let handle = unsafe { mun_type_info_align(type_info, ptr::null_mut()) };
        assert_ne!(handle.0, ptr::null());

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'align' is null pointer."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_type_info_align() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 12345 }
    "#,
        );

        let type_info = get_type_info_by_name(driver.runtime, "core::i32");
        let mut align = MaybeUninit::uninit();
        let handle = unsafe { mun_type_info_align(type_info, align.as_mut_ptr()) };
        assert_eq!(handle.0, ptr::null());

        let align = unsafe { align.assume_init() };
        assert_eq!(align, mem::align_of::<i32>());
    }

    #[test]
    fn test_type_info_data_invalid_type_info() {
        let handle = unsafe { mun_type_info_data(TypeInfoHandle::null(), ptr::null_mut()) };
        assert_ne!(handle.0, ptr::null());

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'type_info' is null pointer."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_type_info_data_invalid_type_info_data() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 12345 }
    "#,
        );

        let type_info = get_type_info_by_name(driver.runtime, "core::i32");
        let handle = unsafe { mun_type_info_data(type_info, ptr::null_mut()) };
        assert_ne!(handle.0, ptr::null());

        let message = unsafe { CStr::from_ptr(handle.0) };
        assert_eq!(
            message.to_str().unwrap(),
            "Invalid argument: 'type_info_data' is null pointer."
        );

        unsafe { mun_error_destroy(handle) };
    }

    #[test]
    fn test_type_info_data_primitive() {
        let driver = TestDriver::new(
            r#"
        pub fn main() -> i32 { 12345 }
    "#,
        );

        let type_info = get_type_info_by_name(driver.runtime, "core::i32");
        let mut type_info_data = MaybeUninit::uninit();
        let handle = unsafe { mun_type_info_data(type_info, type_info_data.as_mut_ptr()) };
        assert_eq!(handle.0, ptr::null());

        let type_info_data = unsafe { type_info_data.assume_init() };
        assert!(type_info_data.is_primitive());
        assert!(!type_info_data.is_struct());
    }

    #[test]
    fn test_type_info_data_struct() {
        let driver = TestDriver::new(
            r#"
            pub struct Foo;

            pub fn main() -> Foo { Foo }
            "#,
        );

        let type_info = get_type_info_by_name(driver.runtime, "Foo");
        let mut type_info_data = MaybeUninit::uninit();
        let handle = unsafe { mun_type_info_data(type_info, type_info_data.as_mut_ptr()) };
        assert_eq!(handle.0, ptr::null());

        let type_info_data = unsafe { type_info_data.assume_init() };
        assert!(!type_info_data.is_primitive());
        assert!(type_info_data.is_struct());
    }

    #[test]
    fn test_type_info_span_destroy() {
        let driver = TestDriver::new(
            r#"
        pub fn add(a: i32, b: i32) -> i32 { a + b }
        pub fn empty() -> i32 { 0 }
    "#,
        );

        let fn_info = get_fake_function_info(driver.runtime, "add");
        let mut arg_types = MaybeUninit::uninit();
        let handle = unsafe { mun_function_info_argument_types(fn_info, arg_types.as_mut_ptr()) };
        assert_eq!(handle.0, ptr::null());

        let arg_types = unsafe { arg_types.assume_init() };
        assert!(unsafe { mun_type_info_span_destroy(arg_types) });

        let fn_info = get_fake_function_info(driver.runtime, "empty");
        let mut arg_types = MaybeUninit::uninit();
        let handle = unsafe { mun_function_info_argument_types(fn_info, arg_types.as_mut_ptr()) };
        assert_eq!(handle.0, ptr::null());

        let arg_types = unsafe { arg_types.assume_init() };
        assert!(!unsafe { mun_type_info_span_destroy(arg_types) });
    }
}

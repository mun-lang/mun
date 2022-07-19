//! Defines an FFI compatible type interface

use std::ops::Deref;
use std::ptr::NonNull;
use std::{
    ffi::c_void, ffi::CString, mem::ManuallyDrop, os::raw::c_char, ptr, sync::atomic::Ordering,
    sync::Arc,
};

use abi::Guid;
use capi_utils::{mun_error_try, try_deref_mut, ErrorHandle};
use r#pointer::PointerType;
use r#struct::StructType;

use crate::r#type::{PointerInfo, StructInfo, TypeInner, TypeInnerData, TypeStore};

mod pointer;
mod primitive;
mod r#struct;

/// A [`Type`] holds information about a mun type.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Type(*const c_void, *const c_void);

impl From<super::Type> for Type {
    fn from(ty: crate::Type) -> Self {
        let ty = ManuallyDrop::new(ty);
        Type(
            ty.inner.as_ptr() as *const _,
            Arc::as_ptr(&ty.store) as *const _,
        )
    }
}

impl Type {
    /// Returns the store associated with the Type or
    unsafe fn store(&self) -> Result<ManuallyDrop<Arc<TypeStore>>, String> {
        if self.1.is_null() {
            return Err(String::from("Type contains invalid pointer"));
        }

        Ok(ManuallyDrop::new(Arc::from_raw(self.1 as *const TypeStore)))
    }

    /// Returns the store associated with the Type or
    unsafe fn inner(&self) -> Result<&TypeInner, String> {
        (self.0 as *const TypeInner)
            .as_ref()
            .ok_or_else(|| String::from("Type contains invalid pointer"))
    }

    /// Converts this FFI type into an owned Rust type. This transfers the ownership from the FFI
    /// type back to Rust.
    unsafe fn to_owned(self) -> Result<super::Type, String> {
        if self.0.is_null() {
            return Err(String::from("Type contains invalid pointer"));
        }

        if self.1.is_null() {
            return Err(String::from("Type contains invalid pointer"));
        }

        Ok(super::Type {
            inner: NonNull::new_unchecked(self.0 as *mut _),
            store: Arc::from_raw(self.1 as *const _),
        })
    }
}

/// Notifies the runtime that the specified type is no longer used. Any use of the type after
/// calling this function results in undefined behavior.
///
/// # Safety
///
/// This function results in undefined behavior if the passed in `Type` has been deallocated in a
/// previous call to [`mun_type_release`].
#[no_mangle]
pub unsafe extern "C" fn mun_type_release(ty: Type) -> ErrorHandle {
    // Transfer ownership to Rust and immediately drop the instance
    let _ = mun_error_try!(ty.to_owned());

    ErrorHandle::default()
}

/// Increments the usage count of the specified type.
///
/// # Safety
///
/// This function results in undefined behavior if the passed in `Type` has been deallocated in a
/// previous call to [`mun_type_release`].
#[no_mangle]
pub unsafe extern "C" fn mun_type_add_reference(ty: Type) -> ErrorHandle {
    let store = mun_error_try!(ty.store());
    let inner = mun_error_try!(ty.inner());

    // Release the references owned by the type
    inner.external_references.fetch_add(1, Ordering::Relaxed);
    Arc::increment_strong_count(Arc::as_ptr(&store));

    ErrorHandle::default()
}

/// Retrieves the type's name.
///
/// # Safety
///
/// The caller is responsible for calling `mun_string_destroy` on the return pointer - if it is not
/// null.
///
/// This function results in undefined behavior if the passed in `Type` has been deallocated in a
/// previous call to [`mun_type_release`].
#[no_mangle]
pub unsafe extern "C" fn mun_type_name(ty: Type) -> *const c_char {
    match ty.inner() {
        Ok(inner) => CString::new(inner.name.clone()).unwrap().into_raw() as *const _,
        Err(_) => ptr::null(),
    }
}

/// Compares two different Types. Returns `true` if the two types are equal. If either of the two
/// types is invalid because for instance it contains null pointers this function returns `false`.
///
/// # Safety
///
/// This function results in undefined behavior if the passed in `Type`s have been deallocated in a
/// previous call to [`mun_type_release`].
#[no_mangle]
pub unsafe extern "C" fn mun_type_equal(a: Type, b: Type) -> bool {
    match (a.inner(), b.inner()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Returns the storage size required for a type. The storage size does not include any padding to
/// align the size. Call [`mun_type_alignment`] to request the alignment of the type.
///
/// # Safety
///
/// This function results in undefined behavior if the passed in `Type`s have been deallocated in a
/// previous call to [`mun_type_release`].
#[no_mangle]
pub unsafe extern "C" fn mun_type_size(ty: Type, size: *mut usize) -> ErrorHandle {
    let size = try_deref_mut!(size);
    let inner = mun_error_try!(ty.inner());
    *size = inner.layout.size();
    ErrorHandle::default()
}

/// Returns the alignment requirements of the type.
///
/// # Safety
///
/// This function results in undefined behavior if the passed in `Type`s have been deallocated in a
/// previous call to [`mun_type_release`].
#[no_mangle]
pub unsafe extern "C" fn mun_type_alignment(ty: Type, align: *mut usize) -> ErrorHandle {
    let align = try_deref_mut!(align);
    let inner = mun_error_try!(ty.inner());
    *align = inner.layout.align();
    ErrorHandle::default()
}

/// Returns a new [`Type`] that is a pointer to the specified type.
///
/// # Safety
///
/// This function results in undefined behavior if the passed in `Type`s have been deallocated in a
/// previous call to [`mun_type_release`].
#[no_mangle]
pub unsafe extern "C" fn mun_type_pointer_type(
    ty: Type,
    mutable: bool,
    pointer_ty: *mut Type,
) -> ErrorHandle {
    let pointer_ty = try_deref_mut!(pointer_ty);
    let store = mun_error_try!(ty.store());
    let inner = mun_error_try!(ty.inner());
    *pointer_ty = inner.pointer_type(mutable, &store).into();
    ErrorHandle::default()
}

/// An enum that defines the kind of type.
#[repr(u8)]
pub enum TypeKind {
    Primitive(Guid),
    Pointer(PointerType),
    Struct(StructType),
}

/// Returns information about what kind of type this is.
///
/// # Safety
///
/// This function results in undefined behavior if the passed in `Type`s have been deallocated in a
/// previous call to [`mun_type_release`].
#[no_mangle]
pub unsafe extern "C" fn mun_type_kind(ty: Type, kind: *mut TypeKind) -> ErrorHandle {
    let kind = try_deref_mut!(kind);
    let store = mun_error_try!(ty.store());
    let inner = mun_error_try!(ty.inner());

    *kind = match &inner.data {
        TypeInnerData::Primitive(guid) => TypeKind::Primitive(*guid),
        TypeInnerData::Pointer(pointer) => TypeKind::Pointer(PointerType(
            (pointer as *const PointerInfo).cast(),
            Arc::as_ptr(ManuallyDrop::deref(&store)) as *const _,
        )),
        TypeInnerData::Struct(s) => TypeKind::Struct(StructType(
            (s as *const StructInfo).cast(),
            Arc::as_ptr(ManuallyDrop::deref(&store)) as *const _,
        )),
        TypeInnerData::Uninitialized => unreachable!(),
    };

    ErrorHandle::default()
}

#[cfg(test)]
mod test {
    use std::ffi::{c_void, CStr, CString};
    use std::mem::MaybeUninit;
    use std::ptr;

    use capi_utils::{assert_error, mun_string_destroy};

    use crate::r#type::ffi::{
        mun_type_add_reference, mun_type_alignment, mun_type_pointer_type, mun_type_size,
    };
    use crate::HasStaticType;

    use super::{
        mun_type_equal, mun_type_name, mun_type_release,
        primitive::{mun_type_primitive, PrimitiveType},
        Type,
    };

    const FFI_TYPE_NULL_INNER: Type = Type(ptr::null(), 0xDEAD as *const c_void);
    const FFI_TYPE_NULL_STORE: Type = Type(0xDEAD as *const c_void, ptr::null());
    const FFI_TYPE_NULL: Type = Type(ptr::null(), ptr::null());

    #[test]
    fn test_mun_type_release_null_type() {
        assert_error!(unsafe { mun_type_release(FFI_TYPE_NULL_INNER) });
        assert_error!(unsafe { mun_type_release(FFI_TYPE_NULL_STORE) });
    }

    #[test]
    fn test_mun_type_add_reference_null_type() {
        assert_error!(unsafe { mun_type_add_reference(FFI_TYPE_NULL_INNER) });
        assert_error!(unsafe { mun_type_add_reference(FFI_TYPE_NULL_STORE) });
    }

    #[test]
    fn test_mun_type_equal() {
        let ffi_f32 = mun_type_primitive(PrimitiveType::F32);
        let ffi_i32 = mun_type_primitive(PrimitiveType::I32);

        assert!(unsafe { mun_type_equal(ffi_f32, ffi_f32) });
        assert!(unsafe { mun_type_equal(ffi_i32, ffi_i32) });
        assert!(!unsafe { mun_type_equal(ffi_f32, ffi_i32) });
        assert!(!unsafe { mun_type_equal(ffi_i32, ffi_f32) });
        assert!(!unsafe { mun_type_equal(ffi_f32, FFI_TYPE_NULL) });
        assert!(!unsafe { mun_type_equal(FFI_TYPE_NULL, ffi_f32) });
        assert!(!unsafe { mun_type_equal(FFI_TYPE_NULL, FFI_TYPE_NULL) });

        unsafe { mun_type_release(ffi_f32) };
        unsafe { mun_type_release(ffi_i32) };
    }

    #[test]
    fn test_mun_type_name() {
        let ffi_f32 = mun_type_primitive(PrimitiveType::F32);
        let ffi_empty = mun_type_primitive(PrimitiveType::Empty);

        let f32_name = unsafe { mun_type_name(ffi_f32) };
        let empty_name = unsafe { mun_type_name(ffi_empty) };

        assert_eq!(
            unsafe { CStr::from_ptr(f32_name) },
            CString::new("core::f32").unwrap().as_ref()
        );
        assert_eq!(
            unsafe { CStr::from_ptr(empty_name) },
            CString::new("core::empty").unwrap().as_ref()
        );

        unsafe { mun_string_destroy(empty_name) };
        unsafe { mun_string_destroy(f32_name) };
        unsafe { mun_type_release(ffi_empty) };
        unsafe { mun_type_release(ffi_f32) };
    }

    #[test]
    fn test_mun_type_name_invalid_type() {
        assert!(unsafe { mun_type_name(FFI_TYPE_NULL) }.is_null());
    }

    #[test]
    fn test_mun_type_size() {
        let ffi_i8 = mun_type_primitive(PrimitiveType::I8);
        let ffi_u16 = mun_type_primitive(PrimitiveType::U16);
        let ffi_i32 = mun_type_primitive(PrimitiveType::I32);
        let ffi_u64 = mun_type_primitive(PrimitiveType::U64);

        assert_size(ffi_i8, 1);
        assert_size(ffi_u16, 2);
        assert_size(ffi_i32, 4);
        assert_size(ffi_u64, 8);

        unsafe { mun_type_release(ffi_u64) };
        unsafe { mun_type_release(ffi_i32) };
        unsafe { mun_type_release(ffi_u16) };
        unsafe { mun_type_release(ffi_i8) };

        fn assert_size(ty: Type, expected_size: usize) {
            let mut size = MaybeUninit::uninit();
            assert!(unsafe { mun_type_size(ty, size.as_mut_ptr()) }.is_ok());
            assert_eq!(unsafe { size.assume_init() }, expected_size);
        }
    }

    #[test]
    fn test_mun_type_size_invalid_null() {
        let mut size = MaybeUninit::uninit();
        assert_error!(unsafe { mun_type_size(FFI_TYPE_NULL, size.as_mut_ptr()) });

        let ffi_i8 = mun_type_primitive(PrimitiveType::I8);
        assert_error!(unsafe { mun_type_size(ffi_i8, ptr::null_mut()) });
        unsafe { mun_type_release(ffi_i8) };
    }

    #[test]
    fn test_mun_type_alignment() {
        let ffi_i8 = mun_type_primitive(PrimitiveType::I8);
        let ffi_u16 = mun_type_primitive(PrimitiveType::U16);
        let ffi_i32 = mun_type_primitive(PrimitiveType::I32);
        let ffi_u64 = mun_type_primitive(PrimitiveType::U64);

        assert_alignment(ffi_i8, 1);
        assert_alignment(ffi_u16, 2);
        assert_alignment(ffi_i32, 4);
        assert_alignment(ffi_u64, 8);

        unsafe { mun_type_release(ffi_u64) };
        unsafe { mun_type_release(ffi_i32) };
        unsafe { mun_type_release(ffi_u16) };
        unsafe { mun_type_release(ffi_i8) };

        fn assert_alignment(ty: Type, expected_alignment: usize) {
            let mut align = MaybeUninit::uninit();
            assert!(unsafe { mun_type_alignment(ty, align.as_mut_ptr()) }.is_ok());
            assert_eq!(unsafe { align.assume_init() }, expected_alignment);
        }
    }

    #[test]
    fn test_mun_type_alignment_invalid_null() {
        let mut size = MaybeUninit::uninit();
        assert_error!(unsafe { mun_type_alignment(FFI_TYPE_NULL, size.as_mut_ptr()) });

        let ffi_i8 = mun_type_primitive(PrimitiveType::I8);
        assert_error!(unsafe { mun_type_alignment(ffi_i8, ptr::null_mut()) });
        unsafe { mun_type_release(ffi_i8) };
    }

    #[test]
    fn test_mun_type_pointer_type() {
        let ffi_u64 = mun_type_primitive(PrimitiveType::U64);

        let mut ffi_u64_pointer = MaybeUninit::uninit();
        assert!(
            unsafe { mun_type_pointer_type(ffi_u64, true, ffi_u64_pointer.as_mut_ptr()) }.is_ok()
        );
        let rust_u64_pointer = unsafe { ffi_u64_pointer.assume_init().to_owned() }.unwrap();
        let pointer_info = rust_u64_pointer
            .as_pointer()
            .expect("type is not a pointer");
        assert_eq!(&pointer_info.pointee(), u64::type_info());
        assert!(&pointer_info.is_mutable());

        unsafe { mun_type_release(ffi_u64) };
    }

    #[test]
    fn test_mun_type_pointer_type_invalid_null() {
        let mut ffi_u64_pointer = MaybeUninit::uninit();
        assert_error!(unsafe {
            mun_type_pointer_type(FFI_TYPE_NULL, true, ffi_u64_pointer.as_mut_ptr())
        });

        let ffi_u64 = mun_type_primitive(PrimitiveType::U64);
        assert_error!(unsafe { mun_type_pointer_type(ffi_u64, true, ptr::null_mut()) });
        unsafe { mun_type_release(ffi_u64) };
    }
}

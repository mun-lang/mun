use std::ffi::c_void;
use std::mem::ManuallyDrop;
use std::ptr;
use std::sync::Arc;

use abi::Guid;
use capi_utils::{mun_error_try, try_deref_mut, ErrorHandle};

use crate::r#type::{StructInfo, TypeStore};
use crate::FieldInfo;

use super::super::{Field as RustField, StructType as RustStructType};

/// Additional information of a struct [`Type`].
///
/// Ownership of this type lies with the [`Type`] that created this instance. As long as the
/// original type is not released through [`mun_type_release`] this type stays alive.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct StructType(*const c_void, *const c_void);

impl<'t> From<RustStructType<'t>> for StructType {
    fn from(ty: RustStructType<'t>) -> Self {
        StructType(
            (ty.inner as *const StructInfo).cast(),
            (ty.store as *const Arc<TypeStore>).cast(),
        )
    }
}

impl StructType {
    /// Returns the struct info associated with the Type
    unsafe fn inner(&self) -> Result<&StructInfo, String> {
        match (self.0 as *const StructInfo).as_ref() {
            Some(store) => Ok(store),
            None => Err(String::from("PointerType contains invalid pointer")),
        }
    }
}

/// Returns the globally unique identifier (GUID) of the struct.
///
/// # Safety
///
/// This function results in undefined behavior if the passed in `StructType` has been deallocated
/// by a previous call to [`mun_type_release`].
#[no_mangle]
pub unsafe extern "C" fn mun_struct_type_guid(ty: StructType, guid: *mut Guid) -> ErrorHandle {
    let ty = mun_error_try!(ty.inner());
    let guid = try_deref_mut!(guid);
    *guid = ty.guid.clone();
    ErrorHandle::default()
}

/// Returns the type of memory management to apply for the struct.
///
/// # Safety
///
/// This function results in undefined behavior if the passed in `StructType` has been deallocated
/// by a previous call to [`mun_type_release`].
#[no_mangle]
pub unsafe extern "C" fn mun_struct_type_memory_kind(
    ty: StructType,
    memory_kind: *mut abi::StructMemoryKind,
) -> ErrorHandle {
    let ty = mun_error_try!(ty.inner());
    let memory_kind = try_deref_mut!(memory_kind);
    *memory_kind = ty.memory_kind;
    ErrorHandle::default()
}

/// An array of [`Field`]s.
///
/// This is backed by a dynamically allocated array. Ownership is transferred via this struct
/// and its contents must be destroyed with [`mun_fields_destroy`].
#[repr(C)]
pub struct Fields {
    fields: *const Field,
    count: usize,
}

/// Destroys the contents of a [`Fields`] struct.
///
/// # Safety
///
/// This function results in undefined behavior if the passed in `Fields` has been deallocated
/// by a previous call to [`mun_fields_destroy`].
#[no_mangle]
pub unsafe extern "C" fn mun_fields_destroy(fields: Fields) -> ErrorHandle {
    if fields.fields.is_null() && fields.count > 0 {
        return ErrorHandle::new("Fields contains invalid pointer");
    } else if fields.count > 0 {
        let _ = Vec::from_raw_parts(fields.fields as *mut _, fields.count, fields.count);
    }
    ErrorHandle::default()
}

/// Retrieves all the fields of the specified struct type.
///
/// # Safety
///
/// This function results in undefined behavior if the passed in `StructType` has been deallocated
/// by a previous call to [`mun_type_release`].
#[no_mangle]
pub unsafe extern "C" fn mun_struct_type_fields(
    ty: StructType,
    fields: *mut Fields,
) -> ErrorHandle {
    let inner = mun_error_try!(ty.inner());
    let fields = try_deref_mut!(fields);
    let mut fields_vec = Vec::from_iter(
        inner
            .fields
            .iter()
            .map(|field| Field((field as *const FieldInfo).cast(), ty.1)),
    );
    fields_vec.shrink_to_fit();
    let mut fields_vec = ManuallyDrop::new(fields_vec);
    debug_assert!(fields_vec.len() == fields_vec.capacity());
    *fields = Fields {
        fields: if fields_vec.is_empty() {
            ptr::null()
        } else {
            fields_vec.as_ptr()
        },
        count: fields_vec.len(),
    };
    ErrorHandle::default()
}

/// Information of a field of a struct [`Type`].
///
/// Ownership of this type lies with the [`Type`] that created this instance. As long as the
/// original type is not released through [`mun_type_release`] this type stays alive.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Field(*const c_void, *const c_void);

impl Field {
    /// Returns the store associated with this instance
    unsafe fn store(&self) -> Result<&Arc<TypeStore>, String> {
        match (self.1 as *const Arc<TypeStore>).as_ref() {
            Some(store) => Ok(store),
            None => Err(String::from("Field contains invalid pointer")),
        }
    }

    /// Returns the field info associated with this instance
    unsafe fn inner(&self) -> Result<&FieldInfo, String> {
        match (self.0 as *const FieldInfo).as_ref() {
            Some(info) => Ok(info),
            None => Err(String::from("Field contains invalid pointer")),
        }
    }
}

#[cfg(test)]
mod test {
    use std::{mem::MaybeUninit, ptr};

    use capi_utils::assert_error;

    use crate::{
        r#type::ffi::mun_type_kind,
        r#type::ffi::r#struct::{mun_struct_type_guid, mun_struct_type_memory_kind},
        HasStaticType, StructTypeBuilder,
    };

    use super::{
        super::{Type, TypeKind},
        StructType,
    };

    unsafe fn struct_type(ty: Type) -> (Type, StructType) {
        let mut ty_kind = MaybeUninit::uninit();
        assert!(mun_type_kind(ty, ty_kind.as_mut_ptr()).is_ok());
        let pointer_ty = match ty_kind.assume_init() {
            TypeKind::Struct(p) => p,
            _ => panic!("invalid type kind for struct"),
        };

        (ty, pointer_ty)
    }

    #[test]
    fn test_mun_struct_type_guid() {
        let rust_ty = StructTypeBuilder::new("Foo")
            .add_field("foo", i32::type_info().clone())
            .finish();

        let guid = rust_ty.as_struct().unwrap().guid().clone();
        let (_ty, struct_ty) = unsafe { struct_type(rust_ty.into()) };

        let mut ffi_guid = MaybeUninit::uninit();
        assert!(unsafe { mun_struct_type_guid(struct_ty, ffi_guid.as_mut_ptr()) }.is_ok());
        let ffi_guid = unsafe { ffi_guid.assume_init() };

        assert_eq!(ffi_guid, guid);
    }

    #[test]
    fn test_mun_struct_type_guid_invalid_null() {
        let mut guid = MaybeUninit::uninit();
        assert_error!(unsafe {
            mun_struct_type_guid(StructType(ptr::null(), ptr::null()), guid.as_mut_ptr())
        });

        let ty = StructTypeBuilder::new("Foo")
            .add_field("foo", i32::type_info().clone())
            .finish()
            .into();
        let (_ty, struct_ty) = unsafe { struct_type(ty) };
        assert_error!(unsafe { mun_struct_type_guid(struct_ty, ptr::null_mut()) });
    }

    #[test]
    fn test_mun_struct_type_memory_kind() {
        let rust_ty = StructTypeBuilder::new("Foo")
            .add_field("foo", i32::type_info().clone())
            .set_memory_kind(abi::StructMemoryKind::Value)
            .finish();

        let (_ty, struct_ty) = unsafe { struct_type(rust_ty.into()) };

        let mut memory_lind = MaybeUninit::uninit();
        assert!(
            unsafe { mun_struct_type_memory_kind(struct_ty, memory_lind.as_mut_ptr()) }.is_ok()
        );
        let memory_lind = unsafe { memory_lind.assume_init() };

        assert_eq!(memory_lind, abi::StructMemoryKind::Value);
    }

    #[test]
    fn test_mun_struct_type_memory_kind_invalid_null() {
        let mut memory_kind = MaybeUninit::uninit();
        assert_error!(unsafe {
            mun_struct_type_memory_kind(
                StructType(ptr::null(), ptr::null()),
                memory_kind.as_mut_ptr(),
            )
        });

        let ty = StructTypeBuilder::new("Foo")
            .add_field("foo", i32::type_info().clone())
            .finish()
            .into();
        let (_ty, struct_ty) = unsafe { struct_type(ty) };
        assert_error!(unsafe { mun_struct_type_memory_kind(struct_ty, ptr::null_mut()) });
    }
}

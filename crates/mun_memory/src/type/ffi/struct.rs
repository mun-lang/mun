use std::ffi::{c_void, CString};
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::os::raw::c_char;
use std::ptr;
use std::sync::Arc;

use abi::Guid;
use capi_utils::{mun_error_try, try_deref_mut, ErrorHandle};

use crate::r#type::ffi::Type;
use crate::r#type::{StructInfo, TypeStore};
use crate::FieldInfo;

use super::super::{StructType as RustStructType, Type as RustType};

/// Additional information of a struct [`Type`].
///
/// Ownership of this type lies with the [`Type`] that created this instance. As long as the
/// original type is not released through [`mun_type_release`] this type stays alive.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct StructType(pub(super) *const c_void, pub(super) *const c_void);

impl<'t> From<RustStructType<'t>> for StructType {
    fn from(ty: RustStructType<'t>) -> Self {
        StructType(
            (ty.inner as *const StructInfo).cast(),
            (&ty.store as *const &Arc<TypeStore>).cast(),
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
        let _ = Vec::from_raw_parts(fields.fields as *mut Field, fields.count, fields.count);
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

    // Get all fields
    let mut fields_vec = Vec::from_iter(
        inner
            .fields
            .iter()
            .map(|field| Field((field as *const FieldInfo).cast(), ty.1)),
    );

    // Ensures that the length and the capacity are the same
    fields_vec.shrink_to_fit();
    debug_assert!(fields_vec.len() == fields_vec.capacity());

    // Transfer ownership over the FFI
    let fields_vec = ManuallyDrop::new(fields_vec);
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
    unsafe fn store(&self) -> Result<ManuallyDrop<Arc<TypeStore>>, String> {
        if self.1.is_null() {
            return Err(String::from("Field contains invalid pointer"));
        }

        Ok(ManuallyDrop::new(Arc::from_raw(self.1 as *const TypeStore)))
    }

    /// Returns the field info associated with this instance
    unsafe fn inner(&self) -> Result<&FieldInfo, String> {
        match (self.0 as *const FieldInfo).as_ref() {
            Some(info) => Ok(info),
            None => Err(String::from("Field contains invalid pointer")),
        }
    }
}

/// Returns the name of the field in the parent struct. Ownership of the name is transferred and
/// must be destroyed with [`mun_string_destroy`]. If this function fails a nullptr is returned.
///
/// # Safety
///
/// This function results in undefined behavior if the passed in `Field` has been deallocated
/// by a previous call to [`mun_type_release`].
#[no_mangle]
pub unsafe extern "C" fn mun_field_name(field: Field, name: *mut *const c_char) -> ErrorHandle {
    let inner = mun_error_try!(field.inner());
    let name = try_deref_mut!(name);
    *name = CString::new(inner.name.clone()).unwrap().into_raw() as *const _;
    ErrorHandle::default()
}

/// Returns the type of the field. Ownership of the returned [`Type`] is transferred and must be
/// released with a call to [`mun_type_release`].
///
/// # Safety
///
/// This function results in undefined behavior if the passed in `Field` has been deallocated
/// by a previous call to [`mun_type_release`].
#[no_mangle]
pub unsafe extern "C" fn mun_field_type(field: Field, ty: *mut Type) -> ErrorHandle {
    let inner = mun_error_try!(field.inner());
    let store = mun_error_try!(field.store());
    let ty = try_deref_mut!(ty);
    *ty = RustType::new_unchecked(inner.type_info, ManuallyDrop::deref(&store).clone()).into();
    ErrorHandle::default()
}

/// Returns the offset of the field in bytes from the start of the parent struct.
///
/// # Safety
///
/// This function results in undefined behavior if the passed in `Field` has been deallocated
/// by a previous call to [`mun_type_release`].
#[no_mangle]
pub unsafe extern "C" fn mun_field_offset(field: Field, offset: *mut usize) -> ErrorHandle {
    let inner = mun_error_try!(field.inner());
    let offset = try_deref_mut!(offset);
    *offset = inner.offset as usize;
    ErrorHandle::default()
}

#[cfg(test)]
mod test {
    use std::ffi::CStr;
    use std::{mem::MaybeUninit, ptr, slice};

    use capi_utils::{assert_error, mun_string_destroy};

    use crate::r#type::ffi::r#struct::mun_field_type;
    use crate::{
        r#type::ffi::{mun_type_kind, mun_type_release},
        HasStaticType, StructTypeBuilder,
    };

    use super::{
        super::{Type, TypeKind},
        mun_field_name, mun_field_offset, mun_fields_destroy, mun_struct_type_fields,
        mun_struct_type_guid, mun_struct_type_memory_kind, Field, StructType,
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
        let (ty, struct_ty) = unsafe { struct_type(rust_ty.into()) };

        let mut ffi_guid = MaybeUninit::uninit();
        assert!(unsafe { mun_struct_type_guid(struct_ty, ffi_guid.as_mut_ptr()) }.is_ok());
        let ffi_guid = unsafe { ffi_guid.assume_init() };

        assert_eq!(ffi_guid, guid);

        assert!(unsafe { mun_type_release(ty) }.is_ok());
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
        let (ty, struct_ty) = unsafe { struct_type(ty) };
        assert_error!(unsafe { mun_struct_type_guid(struct_ty, ptr::null_mut()) });

        assert!(unsafe { mun_type_release(ty) }.is_ok());
    }

    #[test]
    fn test_mun_struct_type_memory_kind() {
        let rust_ty = StructTypeBuilder::new("Foo")
            .add_field("foo", i32::type_info().clone())
            .set_memory_kind(abi::StructMemoryKind::Value)
            .finish();

        let (ty, struct_ty) = unsafe { struct_type(rust_ty.into()) };

        let mut memory_lind = MaybeUninit::uninit();
        assert!(
            unsafe { mun_struct_type_memory_kind(struct_ty, memory_lind.as_mut_ptr()) }.is_ok()
        );
        let memory_lind = unsafe { memory_lind.assume_init() };

        assert_eq!(memory_lind, abi::StructMemoryKind::Value);

        assert!(unsafe { mun_type_release(ty) }.is_ok());
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

    #[test]
    fn test_mun_struct_type_fields() {
        let i32_type = i32::type_info();
        let (struct_with_fields, struct_with_fields_struct) = unsafe {
            struct_type(
                StructTypeBuilder::new("StructWithFields")
                    .add_field("foo", i32_type.clone())
                    .add_field("bar", i32_type.clone())
                    .finish()
                    .into(),
            )
        };

        let mut fields = MaybeUninit::uninit();
        assert!(
            unsafe { mun_struct_type_fields(struct_with_fields_struct, fields.as_mut_ptr()) }
                .is_ok()
        );
        let fields = unsafe { fields.assume_init() };
        assert_eq!(fields.count, 2);

        let fields_slice = unsafe { slice::from_raw_parts(fields.fields, fields.count) };

        let mut field_name = MaybeUninit::uninit();
        let mut field_offset = MaybeUninit::uninit();
        let mut field_type = MaybeUninit::uninit();
        assert!(unsafe { mun_field_name(fields_slice[0], field_name.as_mut_ptr()) }.is_ok());
        assert!(unsafe { mun_field_offset(fields_slice[0], field_offset.as_mut_ptr()) }.is_ok());
        assert!(unsafe { mun_field_type(fields_slice[0], field_type.as_mut_ptr()) }.is_ok());
        let field_name = unsafe { field_name.assume_init() };
        let field_offset = unsafe { field_offset.assume_init() };
        let field_type =
            unsafe { field_type.assume_init().to_owned() }.expect("unable to convert to rust");

        assert_eq!(unsafe { CStr::from_ptr(field_name) }.to_str(), Ok("foo"));
        assert_eq!(field_offset, 0);
        assert_eq!(&field_type, i32::type_info());

        unsafe { mun_string_destroy(field_name) };
        assert!(unsafe { mun_fields_destroy(fields) }.is_ok());
        assert!(unsafe { mun_type_release(struct_with_fields) }.is_ok());
    }

    #[test]
    fn test_mun_struct_type_fields_empty() {
        let (empty_struct, empty_struct_struct) =
            unsafe { struct_type(StructTypeBuilder::new("EmptyStruct").finish().into()) };

        let mut fields = MaybeUninit::uninit();
        assert!(
            unsafe { mun_struct_type_fields(empty_struct_struct, fields.as_mut_ptr()) }.is_ok()
        );
        let fields = unsafe { fields.assume_init() };
        assert_eq!(fields.count, 0);
        assert!(fields.fields.is_null());
        assert!(unsafe { mun_fields_destroy(fields) }.is_ok());

        assert!(unsafe { mun_type_release(empty_struct) }.is_ok());
    }

    #[test]
    fn test_mun_type_name_offset_type_invalid_null() {
        let mut name = MaybeUninit::uninit();
        let mut offset = MaybeUninit::uninit();
        let mut field_type = MaybeUninit::uninit();
        assert_error!(unsafe {
            mun_field_name(Field(ptr::null(), ptr::null()), name.as_mut_ptr())
        });
        assert_error!(unsafe {
            mun_field_offset(Field(ptr::null(), ptr::null()), offset.as_mut_ptr())
        });
        assert_error!(unsafe {
            mun_field_type(Field(ptr::null(), ptr::null()), field_type.as_mut_ptr())
        });

        let ty = StructTypeBuilder::new("Foo")
            .add_field("foo", i32::type_info().clone())
            .finish()
            .into();
        let (ty, struct_ty) = unsafe { struct_type(ty) };
        let mut fields = MaybeUninit::uninit();
        assert!(unsafe { mun_struct_type_fields(struct_ty, fields.as_mut_ptr()) }.is_ok());
        let fields = unsafe { fields.assume_init() };
        assert!(!fields.fields.is_null());

        assert_error!(unsafe { mun_field_name(*fields.fields, ptr::null_mut()) });
        assert_error!(unsafe { mun_field_offset(*fields.fields, ptr::null_mut()) });
        assert_error!(unsafe { mun_field_type(*fields.fields, ptr::null_mut()) });

        assert!(unsafe { mun_fields_destroy(fields) }.is_ok());
        assert!(unsafe { mun_type_release(ty) }.is_ok());
    }
}

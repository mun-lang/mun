use std::{
    ffi::{c_void, CString},
    mem::ManuallyDrop,
    ops::Deref,
    os::raw::c_char,
    ptr, slice,
    sync::Arc,
};

use mun_abi::{self as abi, Guid};
use mun_capi_utils::{mun_error_try, try_deref_mut, ErrorHandle};

use crate::{
    r#type::ffi::Type,
    r#type::{StructData, StructType as RustStructType, Type as RustType, TypeDataStore},
    FieldData,
};

/// Additional information of a struct [`Type`].
///
/// Ownership of this type lies with the [`Type`] that created this instance. As long as the
/// original type is not released through [`mun_type_release`] this type stays alive.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct StructInfo(pub(super) *const c_void, pub(super) *const c_void);

impl<'t> From<RustStructType<'t>> for StructInfo {
    fn from(ty: RustStructType<'t>) -> Self {
        StructInfo(
            (ty.inner as *const StructData).cast(),
            (&ty.store as *const &Arc<TypeDataStore>).cast(),
        )
    }
}

impl StructInfo {
    /// Returns the struct info associated with the Type
    unsafe fn inner(&self) -> Result<&StructData, String> {
        match self.0.cast::<StructData>().as_ref() {
            Some(store) => Ok(store),
            None => Err(String::from("null pointer")),
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
pub unsafe extern "C" fn mun_struct_type_guid(ty: StructInfo, guid: *mut Guid) -> ErrorHandle {
    let ty = mun_error_try!(ty
        .inner()
        .map_err(|e| format!("invalid argument 'ty': {e}")));
    let guid = try_deref_mut!(guid);
    *guid = ty.guid;
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
    ty: StructInfo,
    memory_kind: *mut abi::StructMemoryKind,
) -> ErrorHandle {
    let ty = mun_error_try!(ty
        .inner()
        .map_err(|e| format!("invalid argument 'ty': {e}")));
    let memory_kind = try_deref_mut!(memory_kind);
    *memory_kind = ty.memory_kind;
    ErrorHandle::default()
}

/// An array of [`Field`]s.
///
/// This is backed by a dynamically allocated array. Ownership is transferred via this struct
/// and its contents must be destroyed with [`mun_fields_destroy`].
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Fields {
    pub fields: *const Field,
    pub count: usize,
}

/// Retrieves the field with the given name.
///
/// The name can be passed as a non nul-terminated string it must be UTF-8 encoded.
///
/// # Safety
///
/// This function results in undefined behavior if the passed in `Fields` has been deallocated
/// by a previous call to [`mun_fields_destroy`].
#[no_mangle]
pub unsafe extern "C" fn mun_fields_find_by_name(
    fields: Fields,
    name: *const c_char,
    len: usize,
    has_field: *mut bool,
    field: *mut Field,
) -> ErrorHandle {
    if field.is_null() {
        return ErrorHandle::new("invalid argument 'field': null pointer");
    }
    if name.is_null() {
        return ErrorHandle::new("invalid argument 'name': null pointer");
    };
    let has_field = try_deref_mut!(has_field);
    let field = try_deref_mut!(field);
    let name = std::str::from_utf8_unchecked(slice::from_raw_parts(name.cast::<u8>(), len));

    *has_field = false;

    if fields.fields.is_null() && fields.count == 0 {
        return ErrorHandle::default();
    } else if fields.fields.is_null() {
        return ErrorHandle::new("invalid argument 'fields': invalid pointer");
    }

    let fields = slice::from_raw_parts(fields.fields, fields.count);
    for f in fields {
        let field_inner = mun_error_try!(f.inner());
        if field_inner.name == name {
            *field = *f;
            *has_field = true;
            break;
        }
    }
    ErrorHandle::default()
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
    ty: StructInfo,
    fields: *mut Fields,
) -> ErrorHandle {
    let inner = mun_error_try!(ty
        .inner()
        .map_err(|e| format!("invalid argument 'ty': {e}")));
    let fields = try_deref_mut!(fields);

    // Get all fields
    let mut fields_vec = inner
        .fields
        .iter()
        .map(|field| Field((field as *const FieldData).cast(), ty.1))
        .collect::<Vec<_>>();

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
    unsafe fn store(&self) -> Result<ManuallyDrop<Arc<TypeDataStore>>, String> {
        if self.1.is_null() {
            return Err(String::from("null pointer"));
        }

        Ok(ManuallyDrop::new(Arc::from_raw(
            self.1.cast::<TypeDataStore>(),
        )))
    }

    /// Returns the field info associated with this instance
    unsafe fn inner(&self) -> Result<&FieldData, String> {
        match self.0.cast::<FieldData>().as_ref() {
            Some(info) => Ok(info),
            None => Err(String::from("null pointer")),
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
    let inner = mun_error_try!(field
        .inner()
        .map_err(|e| format!("invalid argument 'ty': {e}")));
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
    let inner = mun_error_try!(field
        .inner()
        .map_err(|e| format!("invalid argument 'ty': {e}")));
    let store = mun_error_try!(field
        .store()
        .map_err(|e| format!("invalid argument 'ty': {e}")));
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
    let inner = mun_error_try!(field
        .inner()
        .map_err(|e| format!("invalid argument 'ty': {e}")));
    let offset = try_deref_mut!(offset);
    *offset = inner.offset as usize;
    ErrorHandle::default()
}

#[cfg(test)]
mod test {
    use std::ffi::{CStr, CString};
    use std::{mem::MaybeUninit, ptr, slice};

    use mun_abi as abi;
    use mun_capi_utils::{
        assert_error_snapshot, assert_getter1, assert_getter3, mun_string_destroy,
    };

    use crate::r#type::ffi::r#struct::mun_fields_find_by_name;
    use crate::{HasStaticType, StructTypeBuilder};

    use super::{
        super::{mun_type_kind, mun_type_release, Type, TypeKind},
        mun_field_name, mun_field_offset, mun_field_type, mun_fields_destroy,
        mun_struct_type_fields, mun_struct_type_guid, mun_struct_type_memory_kind, Field, Fields,
        StructInfo,
    };

    unsafe fn struct_type(ty: Type) -> (Type, StructInfo) {
        assert_getter1!(mun_type_kind(ty, ty_kind));
        let pointer_ty = match ty_kind {
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

        let guid = *rust_ty.as_struct().unwrap().guid();
        let (ty, struct_ty) = unsafe { struct_type(rust_ty.into()) };

        assert_getter1!(mun_struct_type_guid(struct_ty, ffi_guid));
        assert_eq!(ffi_guid, guid);

        assert!(unsafe { mun_type_release(ty) }.is_ok());
    }

    #[test]
    fn test_mun_struct_type_guid_invalid_null() {
        let mut guid = MaybeUninit::uninit();
        assert_error_snapshot!(
            unsafe {
                mun_struct_type_guid(StructInfo(ptr::null(), ptr::null()), guid.as_mut_ptr())
            },
            @r###""invalid argument \'ty\': null pointer""###
        );

        let ty = StructTypeBuilder::new("Foo")
            .add_field("foo", i32::type_info().clone())
            .finish()
            .into();
        let (ty, struct_ty) = unsafe { struct_type(ty) };
        assert_error_snapshot!(
            unsafe { mun_struct_type_guid(struct_ty, ptr::null_mut()) },
            @r###""invalid argument \'guid\': null pointer""###
        );

        assert!(unsafe { mun_type_release(ty) }.is_ok());
    }

    #[test]
    fn test_mun_struct_type_memory_kind() {
        let rust_ty = StructTypeBuilder::new("Foo")
            .add_field("foo", i32::type_info().clone())
            .set_memory_kind(abi::StructMemoryKind::Value)
            .finish();

        let (ty, struct_ty) = unsafe { struct_type(rust_ty.into()) };

        assert_getter1!(mun_struct_type_memory_kind(struct_ty, memory_kind));
        assert_eq!(memory_kind, abi::StructMemoryKind::Value);

        assert!(unsafe { mun_type_release(ty) }.is_ok());
    }

    #[test]
    fn test_mun_struct_type_memory_kind_invalid_null() {
        let mut memory_kind = MaybeUninit::uninit();
        assert_error_snapshot!(unsafe {
            mun_struct_type_memory_kind(
                StructInfo(ptr::null(), ptr::null()),
                memory_kind.as_mut_ptr(),
            )},
            @r###""invalid argument \'ty\': null pointer""###
        );

        let ty = StructTypeBuilder::new("Foo")
            .add_field("foo", i32::type_info().clone())
            .finish()
            .into();
        let (_ty, struct_ty) = unsafe { struct_type(ty) };
        assert_error_snapshot!(
            unsafe { mun_struct_type_memory_kind(struct_ty, ptr::null_mut()) },
            @r###""invalid argument \'memory_kind\': null pointer""###
        );
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

        assert_getter1!(mun_struct_type_fields(struct_with_fields_struct, fields));
        assert_eq!(fields.count, 2);

        let fields_slice = unsafe { slice::from_raw_parts(fields.fields, fields.count) };

        assert_getter1!(mun_field_name(fields_slice[0], field_name));
        assert_getter1!(mun_field_offset(fields_slice[0], field_offset));
        assert_getter1!(mun_field_type(fields_slice[0], field_type));
        let field_type = unsafe { field_type.to_owned() }.expect("unable to convert to rust");

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

        assert_getter1!(mun_struct_type_fields(empty_struct_struct, fields));
        assert_eq!(fields.count, 0);
        assert!(fields.fields.is_null());
        assert!(unsafe { mun_fields_destroy(fields) }.is_ok());

        assert!(unsafe { mun_type_release(empty_struct) }.is_ok());
    }

    #[test]
    fn test_mun_fields_find_by_name() {
        let (_foo_type, foo_struct) = unsafe {
            struct_type(
                StructTypeBuilder::new("Foo")
                    .add_field("hello", i32::type_info().clone())
                    .finish()
                    .into(),
            )
        };

        assert_getter1!(mun_struct_type_fields(foo_struct, fields));

        let hello_str = CString::new("hello").unwrap();
        let world_str = CString::new("world").unwrap();

        assert_getter3!(mun_fields_find_by_name(
            fields,
            hello_str.as_c_str().as_ptr(),
            hello_str.as_bytes().len(),
            has_field,
            _hello_field
        ));
        assert!(has_field);

        assert_getter3!(mun_fields_find_by_name(
            fields,
            world_str.as_c_str().as_ptr(),
            world_str.as_bytes().len(),
            has_field,
            _world_field
        ));
        assert!(!has_field);

        assert_getter3!(mun_fields_find_by_name(
            Fields {
                fields: ptr::null(),
                count: 0
            },
            world_str.as_c_str().as_ptr(),
            world_str.as_bytes().len(),
            has_field,
            _world_field
        ));
        assert!(!has_field);
    }

    #[test]
    fn test_mun_fields_find_by_name_invalid() {
        let (_empty_struct, empty_struct_struct) =
            unsafe { struct_type(StructTypeBuilder::new("EmptyStruct").finish().into()) };

        let name = CString::new("hello").unwrap();

        assert_getter1!(mun_struct_type_fields(empty_struct_struct, fields));

        let mut has_field = MaybeUninit::uninit();
        let mut found_field = MaybeUninit::uninit();
        assert_error_snapshot!(unsafe {
            mun_fields_find_by_name(Fields { fields: ptr::null(), count: 10}, name.as_c_str().as_ptr(), name.as_bytes().len(), has_field.as_mut_ptr(), found_field.as_mut_ptr())
        }, @r###""invalid argument \'fields\': invalid pointer""###);
        assert_error_snapshot!(unsafe {
            mun_fields_find_by_name(fields, ptr::null(), 0, has_field.as_mut_ptr(), found_field.as_mut_ptr())
        }, @r###""invalid argument \'name\': null pointer""###);
        assert_error_snapshot!(unsafe {
            mun_fields_find_by_name(fields, name.as_c_str().as_ptr(), name.as_bytes().len(), has_field.as_mut_ptr(), ptr::null_mut())
        }, @r###""invalid argument \'field\': null pointer""###);
    }

    #[test]
    fn test_mun_type_name_offset_type_invalid_null() {
        let mut name = MaybeUninit::uninit();
        let mut offset = MaybeUninit::uninit();
        let mut field_type = MaybeUninit::uninit();
        assert_error_snapshot!(unsafe {
            mun_field_name(Field(ptr::null(), ptr::null()), name.as_mut_ptr())
        }, @r###""invalid argument \'ty\': null pointer""###);
        assert_error_snapshot!(unsafe {
            mun_field_offset(Field(ptr::null(), ptr::null()), offset.as_mut_ptr())
        }, @r###""invalid argument \'ty\': null pointer""###);
        assert_error_snapshot!(unsafe {
            mun_field_type(Field(ptr::null(), ptr::null()), field_type.as_mut_ptr())
        }, @r###""invalid argument \'ty\': null pointer""###);

        let ty = StructTypeBuilder::new("Foo")
            .add_field("foo", i32::type_info().clone())
            .finish()
            .into();
        let (ty, struct_ty) = unsafe { struct_type(ty) };
        assert_getter1!(mun_struct_type_fields(struct_ty, fields));
        assert!(!fields.fields.is_null());

        assert_error_snapshot!(
            unsafe { mun_field_name(*fields.fields, ptr::null_mut()) },
            @r###""invalid argument \'name\': null pointer""###
        );
        assert_error_snapshot!(
            unsafe { mun_field_offset(*fields.fields, ptr::null_mut()) },
            @r###""invalid argument \'offset\': null pointer""###
        );
        assert_error_snapshot!(
            unsafe { mun_field_type(*fields.fields, ptr::null_mut()) },
            @r###""invalid argument \'ty\': null pointer""###
        );

        assert!(unsafe { mun_fields_destroy(fields) }.is_ok());
        assert!(unsafe { mun_type_release(ty) }.is_ok());
    }
}

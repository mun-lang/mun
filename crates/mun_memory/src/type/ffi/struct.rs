use std::ffi::c_void;
use std::sync::Arc;
use abi::Guid;
use capi_utils::{ErrorHandle, mun_error_try, try_deref_mut};

use crate::r#type::{StructInfo, TypeStore};

use super::super::StructType as RustStructType;

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
    /// Returns the store associated with the Type
    unsafe fn store(&self) -> Result<&Arc<TypeStore>, String> {
        match (self.1 as *const Arc<TypeStore>).as_ref() {
            Some(store) => Ok(store),
            None => Err(String::from("PointerType contains invalid pointer")),
        }
    }

    /// Returns the struct info associated with the Type
    unsafe fn inner(&self) -> Result<&StructInfo, String> {
        match (self.0 as *const StructInfo).as_ref() {
            Some(store) => Ok(store),
            None => Err(String::from("PointerType contains invalid pointer")),
        }
    }

    /// Converts from C FFI type to a Rust type.
    unsafe fn to_rust(&self) -> Result<RustStructType<'_>, String> {
        match (
            (self.0 as *const StructInfo).as_ref(),
            (self.1 as *const Arc<TypeStore>).as_ref(),
        ) {
            (Some(inner), Some(store)) => Ok(RustStructType { inner, store }),
            _ => Err(String::from("RustStructType contains invalid pointer")),
        }
    }
}

/// Returns the globally unique identifier (GUID) of the struct.
///
/// # Safety
///
/// This function results in undefined behavior if the passed in `StructType` has been deallocated
/// by a previous call to [`mun_type_release`].
pub unsafe extern "C" fn mun_struct_type_guid(
    ty: StructType,
    guid: *mut Guid,
) -> ErrorHandle {
    let ty = mun_error_try!(ty.inner());
    let guid = try_deref_mut!(guid);
    *guid = ty.guid.clone();
    ErrorHandle::default()
}

#[cfg(test)]
mod test {
    use std::mem::MaybeUninit;
    use crate::r#type::ffi::mun_type_kind;
    use crate::{StructTypeBuilder};
    use super::{
        StructType,
        super::{Type, TypeKind}
    };

    fn struct_type(builder:StructTypeBuilder) -> (Type, StructType) {
        let ty: Type = builder.finish().into();

        let mut ty_kind = MaybeUninit::uninit();
        assert!(unsafe { mun_type_kind(ty, ty_kind.as_mut_ptr()) }.is_ok());
        let pointer_ty = match unsafe { ty_kind.assume_init() } {
            TypeKind::Struct(p) => p,
            _ => panic!("invalid type kind for pointer")
        };

        (ty, pointer_ty)
    }

    #[test]
    fn test_mun_struct_type_guid() {
        let (ty, struct_ty) = struct_type(StructTypeBuilder::new("Foo").add_field("foo", i32::type))
    }
}

use crate::ffi::Type;
use crate::r#type::{ArrayData, Type as RustType, TypeDataStore};
use mun_capi_utils::{mun_error_try, try_deref_mut, ErrorHandle};
use std::ffi::c_void;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::sync::Arc;

/// Additional information of an array [`Type`].
///
/// Ownership of this type lies with the [`Type`] that created this instance. As long as the
/// original type is not released through [`mun_type_release`] this type stays alive.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ArrayInfo(pub(super) *const c_void, pub(super) *const c_void);

impl<'t> From<crate::ArrayType<'t>> for ArrayInfo {
    fn from(ty: crate::ArrayType<'t>) -> Self {
        ArrayInfo(
            (ty.inner as *const ArrayData).cast(),
            (&ty.store as *const &Arc<TypeDataStore>).cast(),
        )
    }
}

impl ArrayInfo {
    /// Returns the store associated with this instance
    unsafe fn store(&self) -> Result<ManuallyDrop<Arc<TypeDataStore>>, String> {
        if self.1.is_null() {
            return Err(String::from("null pointer"));
        }

        Ok(ManuallyDrop::new(Arc::from_raw(
            self.1 as *const TypeDataStore,
        )))
    }

    /// Returns the struct info associated with the Type
    unsafe fn inner(&self) -> Result<&ArrayData, String> {
        match (self.0 as *const ArrayData).as_ref() {
            Some(store) => Ok(store),
            None => Err(String::from("null pointer")),
        }
    }
}

/// Returns the type of the elements stored in this type. Ownership is transferred if this function
/// returns successfully.
///
/// # Safety
///
/// This function results in undefined behavior if the passed in `ArrayInfo` has been deallocated
/// by a previous call to [`mun_type_release`].
#[no_mangle]
pub unsafe extern "C" fn mun_array_type_element_type(
    ty: ArrayInfo,
    element_ty: *mut Type,
) -> ErrorHandle {
    let store = mun_error_try!(ty
        .store()
        .map_err(|e| format!("invalid argument 'ty': {e}")));
    let ty = mun_error_try!(ty
        .inner()
        .map_err(|e| format!("invalid argument 'ty': {e}")));
    let element_ty = try_deref_mut!(element_ty);
    *element_ty =
        RustType::new_unchecked(ty.element_ty, ManuallyDrop::deref(&store).clone()).into();
    ErrorHandle::default()
}

#[cfg(test)]
mod test {
    use super::{mun_array_type_element_type, ArrayInfo};
    use crate::ffi::{
        mun_type_array_type, mun_type_equal, mun_type_kind, mun_type_release, Type, TypeKind,
    };
    use crate::r#type::ffi::primitive::{mun_type_primitive, PrimitiveType};
    use mun_capi_utils::{assert_error_snapshot, assert_getter1};
    use std::{mem::MaybeUninit, ptr};

    /// Returns the array type of the specified type. Asserts if that fails.
    unsafe fn array_type(ty: Type) -> (Type, ArrayInfo) {
        assert_getter1!(mun_type_array_type(ty, array_ty));

        assert_getter1!(mun_type_kind(array_ty, ty_kind));
        let array_ty = match ty_kind {
            TypeKind::Array(a) => a,
            _ => panic!("invalid type kind for array"),
        };

        (ty, array_ty)
    }

    #[test]
    fn test_mun_array_type_pointee() {
        let ffi_f32 = mun_type_primitive(PrimitiveType::F32);
        let (ffi_f32_ptr, array_info) = unsafe { array_type(ffi_f32) };

        assert_getter1!(mun_array_type_element_type(array_info, element_ty));
        assert!(unsafe { mun_type_equal(element_ty, ffi_f32) });

        unsafe { mun_type_release(element_ty) };
        unsafe { mun_type_release(ffi_f32_ptr) };
        unsafe { mun_type_release(ffi_f32) };
    }

    #[test]
    fn test_mun_array_type_pointee_invalid_null() {
        let mut pointee_ty = MaybeUninit::uninit();
        assert_error_snapshot!(
            unsafe {
                mun_array_type_element_type(
                    ArrayInfo(ptr::null(), ptr::null()),
                    pointee_ty.as_mut_ptr(),
                )
            },
            @r###""invalid argument \'ty\': null pointer""###
        );

        let ffi_f32 = mun_type_primitive(PrimitiveType::F32);
        let (ffi_f32_ptr, ptr_info) = unsafe { array_type(ffi_f32) };
        assert_error_snapshot!(
            unsafe { mun_array_type_element_type(ptr_info, ptr::null_mut()) },
            @r###""invalid argument \'element_ty\': null pointer""###
        );

        unsafe { mun_type_release(ffi_f32_ptr) };
        unsafe { mun_type_release(ffi_f32) };
    }
}

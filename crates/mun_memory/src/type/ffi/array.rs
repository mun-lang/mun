use crate::r#type::{ArrayData, TypeDataStore};
use std::ffi::c_void;
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
    /// Returns the struct info associated with the Type
    unsafe fn inner(&self) -> Result<&ArrayData, String> {
        match (self.0 as *const ArrayData).as_ref() {
            Some(store) => Ok(store),
            None => Err(String::from("null pointer")),
        }
    }
}

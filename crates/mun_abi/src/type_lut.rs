use std::ffi::CStr;
use std::hash::Hash;
use std::os::raw::c_char;
use std::{ffi, fmt, slice, str};

use itertools::izip;

use crate::Guid;

/// Represents a unique identifier for types. The runtime can use this to lookup the corresponding
/// [`TypeInfo`]. A [`TypeId`] is a key for a [`TypeInfo`].
///
/// A [`TypeId`] only contains enough information to query the runtime for a concrete type.
#[repr(C)]
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum TypeId<'a> {
    /// Represents a concrete type with a specific Guid
    Concrete(Guid),

    /// Represents a pointer to a type
    Pointer(PointerTypeId<'a>),
}

/// Represents a pointer to another type.
#[repr(C)]
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct PointerTypeId<'a> {
    /// The type to which this pointer points
    pub pointee: &'a TypeId<'a>,

    /// Whether or not this pointer is mutable or not
    pub mutable: bool,
}

unsafe impl<'a> Send for TypeId<'a> {}
unsafe impl<'a> Sync for TypeId<'a> {}

impl<'a> From<Guid> for TypeId<'a> {
    fn from(guid: Guid) -> Self {
        TypeId::Concrete(guid)
    }
}

impl<'a> From<PointerTypeId<'a>> for TypeId<'a> {
    fn from(pointer: PointerTypeId<'a>) -> Self {
        TypeId::Pointer(pointer)
    }
}

impl<'a> fmt::Display for TypeId<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeId::Concrete(guid) => guid.fmt(f),
            TypeId::Pointer(pointer) => pointer.fmt(f),
        }
    }
}

impl<'a> fmt::Display for PointerTypeId<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.mutable {
            write!(f, "*mut ")
        } else {
            write!(f, "*const ")
        }?;
        self.pointee.fmt(f)
    }
}

/// Represents a lookup table for type information. This is used for runtime linking.
///
/// Type IDs and handles are stored separately for cache efficiency.
#[repr(C)]
pub struct TypeLut<'a> {
    /// Type IDs
    pub(crate) type_ids: *const TypeId<'a>,
    /// Type information handles
    pub(crate) type_handles: *mut *const ffi::c_void,
    /// Debug names
    pub(crate) type_names: *const *const c_char,
    /// Number of types
    pub num_entries: u32,
}

impl<'a> TypeLut<'a> {
    /// Returns an iterator over pairs of type IDs and type handles.
    pub fn iter(&self) -> impl Iterator<Item = (&TypeId, &*const ffi::c_void, &str)> {
        let (type_ids, type_ptrs, type_names) = if self.num_entries != 0 {
            let ptrs =
                unsafe { slice::from_raw_parts_mut(self.type_handles, self.num_entries as usize) };
            let type_ids =
                unsafe { slice::from_raw_parts(self.type_ids, self.num_entries as usize) };
            let type_names =
                unsafe { slice::from_raw_parts(self.type_names, self.num_entries as usize) };

            (type_ids.iter(), ptrs.iter(), type_names.iter())
        } else {
            ((&[]).iter(), (&[]).iter(), (&[]).iter())
        };

        izip!(type_ids, type_ptrs, type_names).map(|(id, ptr, type_name)| {
            (id, ptr, unsafe {
                std::str::from_utf8_unchecked(CStr::from_ptr(*type_name).to_bytes())
            })
        })
    }

    /// Returns an iterator over pairs of type IDs and mutable type handles.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&TypeId, &mut *const ffi::c_void, &str)> {
        let (type_ids, type_ptrs, type_names) = if self.num_entries != 0 {
            let ptrs =
                unsafe { slice::from_raw_parts_mut(self.type_handles, self.num_entries as usize) };
            let type_ids =
                unsafe { slice::from_raw_parts(self.type_ids, self.num_entries as usize) };
            let type_names =
                unsafe { slice::from_raw_parts(self.type_names, self.num_entries as usize) };

            (type_ids.iter(), ptrs.iter_mut(), type_names.iter())
        } else {
            ((&[]).iter(), (&mut []).iter_mut(), (&[]).iter())
        };

        izip!(type_ids, type_ptrs, type_names).map(|(id, ptr, type_name)| {
            (id, ptr, unsafe {
                std::str::from_utf8_unchecked(CStr::from_ptr(*type_name).to_bytes())
            })
        })
    }

    /// Returns mutable type handles.
    pub fn type_handles_mut(&mut self) -> &mut [*const ffi::c_void] {
        if self.num_entries == 0 {
            &mut []
        } else {
            unsafe { slice::from_raw_parts_mut(self.type_handles, self.num_entries as usize) }
        }
    }

    /// Returns type IDs.
    pub fn type_ids(&self) -> &[TypeId<'a>] {
        if self.num_entries == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(self.type_ids, self.num_entries as usize) }
        }
    }

    /// Returns a type handle, without doing bounds checking.
    ///
    /// This is generally not recommended, use with caution! Calling this method with an
    /// out-of-bounds index is _undefined behavior_ even if the resulting reference is not used.
    /// For a safe alternative see [get_ptr](#method.get_ptr).
    ///
    /// # Safety
    ///
    /// The `idx` is not bounds checked and should therefor be used with care.
    pub unsafe fn get_type_handle_unchecked(&self, idx: u32) -> *const ffi::c_void {
        *self.type_handles.offset(idx as isize)
    }

    /// Returns a type handle at the given index, or `None` if out of bounds.
    pub fn get_type_handle(&self, idx: u32) -> Option<*const ffi::c_void> {
        if idx < self.num_entries {
            Some(unsafe { self.get_type_handle_unchecked(idx) })
        } else {
            None
        }
    }

    /// Returns a mutable reference to a type handle, without doing bounds checking.
    ///
    /// This is generally not recommended, use with caution! Calling this method with an
    /// out-of-bounds index is _undefined behavior_ even if the resulting reference is not used.
    /// For a safe alternative see [get_ptr_mut](#method.get_ptr_mut).
    ///
    /// # Safety
    ///
    /// The `idx` is not bounds checked and should therefor be used with care.
    pub unsafe fn get_type_handle_unchecked_mut(&mut self, idx: u32) -> &mut *const ffi::c_void {
        &mut *self.type_handles.offset(idx as isize)
    }

    /// Returns a mutable reference to a type handle at the given index, or `None` if out of
    /// bounds.
    pub fn get_type_handle_mut(&mut self, idx: u32) -> Option<&mut *const ffi::c_void> {
        if idx < self.num_entries {
            Some(unsafe { self.get_type_handle_unchecked_mut(idx) })
        } else {
            None
        }
    }

    /// Returns type names.
    pub fn type_names(&self) -> impl Iterator<Item = &str> {
        let type_names = if self.num_entries == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(self.type_names, self.num_entries as usize) }
        };

        type_names
            .iter()
            .map(|n| unsafe { str::from_utf8_unchecked(CStr::from_ptr(*n).to_bytes()) })
    }
}

#[cfg(test)]
mod tests {
    use std::{ffi::CString, ptr};

    use crate::test_utils::{fake_type_lut, FAKE_TYPE_ID, FAKE_TYPE_NAME};

    #[test]
    fn test_type_lut_iter_mut_none() {
        let type_ids = &[];
        let type_ptrs = &mut [];
        let type_names = &[];
        let mut type_lut = fake_type_lut(type_ids, type_ptrs, type_names);

        let iter = type_ids.iter().zip(type_ptrs.iter_mut());
        assert_eq!(type_lut.iter_mut().count(), iter.count());
    }

    #[test]
    fn test_type_lut_iter_mut_some() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");

        let type_ids = &[FAKE_TYPE_ID];
        let type_ptrs = &mut [ptr::null()];
        let type_names = &[type_name.as_ptr()];
        let mut type_lut = fake_type_lut(type_ids, type_ptrs, type_names);

        let iter = type_ids.iter().zip(type_ptrs.iter_mut());
        assert_eq!(type_lut.iter_mut().count(), iter.len());

        for (lhs, rhs) in type_lut.iter_mut().zip(iter) {
            assert_eq!(lhs.0, rhs.0);
            assert_eq!(lhs.1, rhs.1);
        }
    }

    #[test]
    fn test_type_lut_ptrs_mut_none() {
        let type_ids = &[];
        let type_ptrs = &mut [];
        let type_names = &[];
        let mut type_lut = fake_type_lut(type_ids, type_ptrs, type_names);

        assert_eq!(type_lut.type_handles_mut().len(), 0);
    }

    #[test]
    fn test_type_lut_ptrs_mut_some() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");

        let type_ids = &[FAKE_TYPE_ID];
        let type_ptrs = &mut [ptr::null()];
        let type_names = &[type_name.as_ptr()];
        let mut type_lut = fake_type_lut(type_ids, type_ptrs, type_names);

        let result = type_lut.type_handles_mut();
        assert_eq!(result.len(), type_ptrs.len());
        for (lhs, rhs) in result.iter().zip(type_ptrs.iter()) {
            assert_eq!(lhs, rhs);
        }
    }

    #[test]
    fn test_type_lut_type_ids_none() {
        let type_ids = &[];
        let type_ptrs = &mut [];
        let type_names = &[];
        let type_lut = fake_type_lut(type_ids, type_ptrs, type_names);

        assert_eq!(type_lut.type_ids().len(), 0);
    }

    #[test]
    fn test_type_lut_type_ids_some() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");

        let type_ids = &[FAKE_TYPE_ID];
        let type_ptrs = &mut [ptr::null()];
        let type_names = &[type_name.as_ptr()];
        let type_lut = fake_type_lut(type_ids, type_ptrs, type_names);

        let result = type_lut.type_ids();
        assert_eq!(result.len(), type_ids.len());
        for (lhs, rhs) in result.iter().zip(type_ids.iter()) {
            assert_eq!(lhs, rhs);
        }
    }

    #[test]
    fn test_type_lut_get_ptr_unchecked() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");

        let type_ids = &[FAKE_TYPE_ID];
        let type_ptrs = &mut [ptr::null()];
        let type_names = &[type_name.as_ptr()];

        let type_lut = fake_type_lut(type_ids, type_ptrs, type_names);
        assert_eq!(
            unsafe { type_lut.get_type_handle_unchecked(0) },
            type_ptrs[0]
        );
    }

    #[test]
    fn test_type_lut_get_ptr_none() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");

        let prototype = &[FAKE_TYPE_ID];
        let type_ptrs = &mut [ptr::null()];
        let type_names = &[type_name.as_ptr()];

        let type_lut = fake_type_lut(prototype, type_ptrs, type_names);
        assert_eq!(type_lut.get_type_handle(1), None);
    }

    #[test]
    fn test_type_lut_get_ptr_some() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");

        let type_ids = &[FAKE_TYPE_ID];
        let type_ptrs = &mut [ptr::null()];
        let type_names = &[type_name.as_ptr()];

        let type_lut = fake_type_lut(type_ids, type_ptrs, type_names);
        assert_eq!(type_lut.get_type_handle(0), Some(type_ptrs[0]));
    }

    #[test]
    fn test_type_lut_get_ptr_unchecked_mut() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");

        let type_ids = &[FAKE_TYPE_ID];
        let type_ptrs = &mut [ptr::null()];
        let type_names = &[type_name.as_ptr()];

        let mut type_lut = fake_type_lut(type_ids, type_ptrs, type_names);
        assert_eq!(
            unsafe { type_lut.get_type_handle_unchecked_mut(0) },
            &mut type_ptrs[0]
        );
    }

    #[test]
    fn test_type_lut_get_ptr_mut_none() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");

        let type_ids = &[FAKE_TYPE_ID];
        let type_ptrs = &mut [ptr::null()];
        let type_names = &[type_name.as_ptr()];

        let mut type_lut = fake_type_lut(type_ids, type_ptrs, type_names);
        assert_eq!(type_lut.get_type_handle_mut(1), None);
    }

    #[test]
    fn test_type_lut_get_ptr_mut_some() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");

        let type_ids = &[FAKE_TYPE_ID];
        let type_ptrs = &mut [ptr::null()];
        let type_names = &[type_name.as_ptr()];

        let mut type_lut = fake_type_lut(type_ids, type_ptrs, type_names);
        assert_eq!(type_lut.get_type_handle_mut(0), Some(&mut type_ptrs[0]));
    }

    #[test]
    fn test_type_lut_type_names_none() {
        let type_ids = &[];
        let type_ptrs = &mut [];
        let type_names = &[];
        let type_lut = fake_type_lut(type_ids, type_ptrs, type_names);

        assert_eq!(type_lut.type_names().count(), 0);
    }

    #[test]
    fn test_type_lut_type_names_some() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");

        let type_ids = &[FAKE_TYPE_ID];
        let type_ptrs = &mut [ptr::null()];
        let type_names = &[type_name.as_ptr()];
        let type_lut = fake_type_lut(type_ids, type_ptrs, type_names);

        for (lhs, rhs) in type_lut.type_names().zip([FAKE_TYPE_NAME].iter()) {
            assert_eq!(lhs, *rhs)
        }
    }
}

use crate::FunctionPrototype;
use std::{ffi::c_void, slice};

/// Represents a function dispatch table. This is used for runtime linking.
///
/// Function signatures and pointers are stored separately for cache efficiency.
#[repr(C)]
pub struct DispatchTable {
    /// Function signatures
    pub(crate) prototypes: *const FunctionPrototype,
    /// Function pointers
    pub(crate) fn_ptrs: *mut *const c_void,
    /// Number of functions
    pub num_entries: u32,
}

impl DispatchTable {
    /// Returns an iterator over pairs of mutable function pointers and signatures.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&mut *const c_void, &FunctionPrototype)> {
        if self.num_entries == 0 {
            (&mut []).iter_mut().zip((&[]).iter())
        } else {
            let ptrs =
                unsafe { slice::from_raw_parts_mut(self.fn_ptrs, self.num_entries as usize) };
            let signatures =
                unsafe { slice::from_raw_parts(self.prototypes, self.num_entries as usize) };

            ptrs.iter_mut().zip(signatures.iter())
        }
    }

    /// Returns an iterator over pairs of function pointers and signatures.
    pub fn iter(&self) -> impl Iterator<Item = (&*const c_void, &FunctionPrototype)> {
        if self.num_entries == 0 {
            (&[]).iter().zip((&[]).iter())
        } else {
            let ptrs =
                unsafe { slice::from_raw_parts_mut(self.fn_ptrs, self.num_entries as usize) };
            let signatures =
                unsafe { slice::from_raw_parts(self.prototypes, self.num_entries as usize) };

            ptrs.iter().zip(signatures.iter())
        }
    }

    /// Returns mutable functions pointers.
    pub fn ptrs_mut(&mut self) -> &mut [*const c_void] {
        if self.num_entries == 0 {
            &mut []
        } else {
            unsafe { slice::from_raw_parts_mut(self.fn_ptrs, self.num_entries as usize) }
        }
    }

    /// Returns function prototypes.
    pub fn prototypes(&self) -> &[FunctionPrototype] {
        if self.num_entries == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(self.prototypes, self.num_entries as usize) }
        }
    }

    /// Returns a function pointer, without doing bounds checking.
    ///
    /// This is generally not recommended, use with caution! Calling this method with an
    /// out-of-bounds index is _undefined behavior_ even if the resulting reference is not used.
    /// For a safe alternative see [get_ptr](#method.get_ptr).
    ///
    /// # Safety
    ///
    /// The `idx` is not bounds checked and should therefor be used with care.
    pub unsafe fn get_ptr_unchecked(&self, idx: u32) -> *const c_void {
        *self.fn_ptrs.offset(idx as isize)
    }

    /// Returns a function pointer at the given index, or `None` if out of bounds.
    pub fn get_ptr(&self, idx: u32) -> Option<*const c_void> {
        if idx < self.num_entries {
            Some(unsafe { self.get_ptr_unchecked(idx) })
        } else {
            None
        }
    }

    /// Returns a mutable reference to a function pointer, without doing bounds checking.
    ///
    /// This is generally not recommended, use with caution! Calling this method with an
    /// out-of-bounds index is _undefined behavior_ even if the resulting reference is not used.
    /// For a safe alternative see [get_ptr_mut](#method.get_ptr_mut).
    ///
    /// # Safety
    ///
    /// The `idx` is not bounds checked and should therefor be used with care.
    pub unsafe fn get_ptr_unchecked_mut(&mut self, idx: u32) -> &mut *const c_void {
        &mut *self.fn_ptrs.offset(idx as isize)
    }

    /// Returns a mutable reference to a function pointer at the given index, or `None` if out of
    /// bounds.
    pub fn get_ptr_mut(&mut self, idx: u32) -> Option<&mut *const c_void> {
        if idx < self.num_entries {
            Some(unsafe { self.get_ptr_unchecked_mut(idx) })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        test_utils::{
            fake_dispatch_table, fake_fn_prototype, fake_type_ref, FAKE_FN_NAME, FAKE_TYPE_NAME,
        },
        TypeRefKindData,
    };
    use std::{ffi::CString, ptr};

    #[test]
    fn test_dispatch_table_iter_mut_none() {
        let signatures = &[];
        let fn_ptrs = &mut [];
        let mut dispatch_table = fake_dispatch_table(signatures, fn_ptrs);

        let iter = fn_ptrs.iter_mut().zip(signatures.iter());
        assert_eq!(dispatch_table.iter_mut().count(), iter.count());
    }

    #[test]
    fn test_dispatch_table_iter_mut_some() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_info = fake_type_ref(&type_name, TypeRefKindData::Primitive);

        let return_type = Some(&type_info);
        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_prototype = fake_fn_prototype(&fn_name, &[], return_type);

        let prototypes = &[fn_prototype];
        let fn_ptrs = &mut [ptr::null()];
        let mut dispatch_table = fake_dispatch_table(prototypes, fn_ptrs);

        let iter = fn_ptrs.iter_mut().zip(prototypes.iter());
        assert_eq!(dispatch_table.iter_mut().count(), iter.len());

        for (lhs, rhs) in dispatch_table.iter_mut().zip(iter) {
            assert_eq!(lhs.0, rhs.0);
            assert_eq!(lhs.1.name(), rhs.1.name());
            assert_eq!(lhs.1.signature.arg_types(), rhs.1.signature.arg_types());
            assert_eq!(lhs.1.signature.return_type(), rhs.1.signature.return_type());
        }
    }

    #[test]
    fn test_dispatch_table_ptrs_mut_none() {
        let signatures = &[];
        let fn_ptrs = &mut [];
        let mut dispatch_table = fake_dispatch_table(signatures, fn_ptrs);

        assert_eq!(dispatch_table.ptrs_mut().len(), 0);
    }

    #[test]
    fn test_dispatch_table_ptrs_mut_some() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_ref = fake_type_ref(&type_name, TypeRefKindData::Primitive);

        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_prototype = fake_fn_prototype(&fn_name, &[], Some(&type_ref));

        let prototypes = &[fn_prototype];
        let fn_ptrs = &mut [ptr::null()];
        let mut dispatch_table = fake_dispatch_table(prototypes, fn_ptrs);

        let result = dispatch_table.ptrs_mut();
        assert_eq!(result.len(), fn_ptrs.len());
        for (lhs, rhs) in result.iter().zip(fn_ptrs.iter()) {
            assert_eq!(lhs, rhs);
        }
    }

    #[test]
    fn test_dispatch_table_signatures_none() {
        let signatures = &[];
        let fn_ptrs = &mut [];
        let dispatch_table = fake_dispatch_table(signatures, fn_ptrs);

        assert_eq!(dispatch_table.prototypes().len(), 0);
    }

    #[test]
    fn test_dispatch_table_signatures_some() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_ref = fake_type_ref(&type_name, TypeRefKindData::Primitive);

        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_prototype = fake_fn_prototype(&fn_name, &[], Some(&type_ref));

        let prototypes = &[fn_prototype];
        let fn_ptrs = &mut [ptr::null()];
        let dispatch_table = fake_dispatch_table(prototypes, fn_ptrs);

        let result = dispatch_table.prototypes();
        assert_eq!(result.len(), prototypes.len());
        for (lhs, rhs) in result.iter().zip(prototypes.iter()) {
            assert_eq!(lhs.name(), rhs.name());
            assert_eq!(lhs.signature.arg_types(), rhs.signature.arg_types());
            assert_eq!(lhs.signature.return_type(), rhs.signature.return_type());
        }
    }

    #[test]
    fn test_dispatch_table_get_ptr_unchecked() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_ref = fake_type_ref(&type_name, TypeRefKindData::Primitive);

        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_prototype = fake_fn_prototype(&fn_name, &[], Some(&type_ref));

        let prototypes = &[fn_prototype];
        let fn_ptrs = &mut [ptr::null()];

        let dispatch_table = fake_dispatch_table(prototypes, fn_ptrs);
        assert_eq!(unsafe { dispatch_table.get_ptr_unchecked(0) }, fn_ptrs[0]);
    }

    #[test]
    fn test_dispatch_table_get_ptr_none() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_ref = fake_type_ref(&type_name, TypeRefKindData::Primitive);

        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_prototype = fake_fn_prototype(&fn_name, &[], Some(&type_ref));

        let prototype = &[fn_prototype];
        let fn_ptrs = &mut [ptr::null()];

        let dispatch_table = fake_dispatch_table(prototype, fn_ptrs);
        assert_eq!(dispatch_table.get_ptr(1), None);
    }

    #[test]
    fn test_dispatch_table_get_ptr_some() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_ref = fake_type_ref(&type_name, TypeRefKindData::Primitive);

        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_prototype = fake_fn_prototype(&fn_name, &[], Some(&type_ref));

        let prototypes = &[fn_prototype];
        let fn_ptrs = &mut [ptr::null()];

        let dispatch_table = fake_dispatch_table(prototypes, fn_ptrs);
        assert_eq!(dispatch_table.get_ptr(0), Some(fn_ptrs[0]));
    }

    #[test]
    fn test_dispatch_table_get_ptr_unchecked_mut() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_ref = fake_type_ref(&type_name, TypeRefKindData::Primitive);

        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_prototype = fake_fn_prototype(&fn_name, &[], Some(&type_ref));

        let prototypes = &[fn_prototype];
        let fn_ptrs = &mut [ptr::null()];

        let mut dispatch_table = fake_dispatch_table(prototypes, fn_ptrs);
        assert_eq!(
            unsafe { dispatch_table.get_ptr_unchecked_mut(0) },
            &mut fn_ptrs[0]
        );
    }

    #[test]
    fn test_dispatch_table_get_ptr_mut_none() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_ref = fake_type_ref(&type_name, TypeRefKindData::Primitive);

        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_prototype = fake_fn_prototype(&fn_name, &[], Some(&type_ref));

        let prototypes = &[fn_prototype];
        let fn_ptrs = &mut [ptr::null()];

        let mut dispatch_table = fake_dispatch_table(prototypes, fn_ptrs);
        assert_eq!(dispatch_table.get_ptr_mut(1), None);
    }

    #[test]
    fn test_dispatch_table_get_ptr_mut_some() {
        let type_name = CString::new(FAKE_TYPE_NAME).expect("Invalid fake type name.");
        let type_ref = fake_type_ref(&type_name, TypeRefKindData::Primitive);

        let fn_name = CString::new(FAKE_FN_NAME).expect("Invalid fake fn name.");
        let fn_prototype = fake_fn_prototype(&fn_name, &[], Some(&type_ref));

        let prototypes = &[fn_prototype];
        let fn_ptrs = &mut [ptr::null()];

        let mut dispatch_table = fake_dispatch_table(prototypes, fn_ptrs);
        assert_eq!(dispatch_table.get_ptr_mut(0), Some(&mut fn_ptrs[0]));
    }
}

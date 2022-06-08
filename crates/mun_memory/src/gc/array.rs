use std::alloc::Layout;
use std::ptr::NonNull;

/// An array in Mun is represented in memory by a header followed by the rest of the bytes.
#[repr(C)]
pub struct Array {
    pub length: usize,
    pub capacity: usize,
}

impl Array {
    /// Returns a pointer to the start of the data of this array
    pub unsafe fn data(&self) -> NonNull<u8> {
        NonNull::new_unchecked((self as *const Array as *mut Array).add(1).cast())
    }
}

use std::ffi;

use crate::ir::dispatch_table::FunctionPrototype;

#[macro_use]
mod macros;

/// Defines the properties of an intrinsic function that can be called from Mun.
/// These functions are mostly used internally.
pub trait Intrinsic: Sync {
    /// Returns the prototype of the intrinsic
    fn prototype(&self) -> FunctionPrototype;
}

intrinsics! {
    /// Allocates memory for the specified `type` in the allocator referred to by `alloc_handle`.
    pub fn new(type_handle: *const ffi::c_void, alloc_handle: *mut ffi::c_void) -> *const *mut ffi::c_void;

    /// Allocates memory for an array of the specified `type` in the allocator referred to by
    /// `alloc_handle` with at least enough capacity to hold `length` elements.
    ///
    /// Note that the elements in the array are left uninitialized.
    pub fn new_array(type_handle: *const ffi::c_void, length: usize, alloc_handle: *mut ffi::c_void) -> *const *mut ffi::c_void;
}

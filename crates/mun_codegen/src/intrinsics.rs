use crate::ir::dispatch_table::FunctionPrototype;
use inkwell::{context::Context, targets::TargetData, types::FunctionType};
use std::ffi;

#[macro_use]
mod macros;

/// Defines the properties of an intrinsic function that can be called from Mun. These functions
/// are mostly used internally.
pub trait Intrinsic: Sync {
    /// Returns the prototype of the intrinsic
    fn prototype(&self) -> FunctionPrototype;

    /// Returns the IR type for the function
    fn ir_type<'ink>(&self, context: &'ink Context, target: &TargetData) -> FunctionType<'ink>;
}

intrinsics! {
    /// Allocates memory for the specified `type` in the allocator referred to by `alloc_handle`.
    pub fn new(type_handle: *const ffi::c_void, alloc_handle: *mut ffi::c_void) -> *const *mut ffi::c_void;
}

use crate::ir::dispatch_table::FunctionPrototype;
use crate::type_info::TypeInfo;
use inkwell::context::Context;
use inkwell::targets::TargetData;
use inkwell::types::FunctionType;
use std::ffi;

#[macro_use]
mod macros;

/// Defines the properties of an intrinsic function that can be called from Mun. These functions
/// are mostly used internally.
pub trait Intrinsic: Sync {
    /// Returns the prototype of the intrinsic
    fn prototype(&self, context: &Context, target: &TargetData) -> FunctionPrototype;

    /// Returns the IR type for the function
    fn ir_type(&self, context: &Context, target: &TargetData) -> FunctionType;
}

intrinsics! {
    /// Allocates memory for the specified `type` in the allocator referred to by `alloc_handle`.
    pub fn new(type: *const TypeInfo, alloc_handle: *mut ffi::c_void) -> *const *mut ffi::c_void;
}

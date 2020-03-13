use crate::ir::dispatch_table::FunctionPrototype;
use crate::type_info::TypeInfo;
use inkwell::context::Context;
use inkwell::types::FunctionType;
use std::ffi;

#[macro_use]
mod macros;

/// Defines the properties of an intrinsic function that can be called from Mun. These functions
/// are mostly used internally.
pub trait Intrinsic: Sync {
    /// Returns the prototype of the intrinsic
    fn prototype(&self) -> FunctionPrototype;

    /// Returns the IR type for the function
    fn ir_type(&self, context: &Context) -> FunctionType;
}

intrinsics! {
    /// Allocates memory for the specified type.
    pub fn new(type: *const TypeInfo, alloc_handle: *mut ffi::c_void) -> *const *mut ffi::c_void;
    /// Allocates memory for and clones the specified type located at `src` into it.
    pub fn clone(src: *const ffi::c_void, alloc_handle: *mut ffi::c_void) -> *const *mut ffi::c_void;
}

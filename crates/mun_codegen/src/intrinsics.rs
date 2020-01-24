use crate::ir::dispatch_table::FunctionPrototype;
use inkwell::context::Context;
use inkwell::types::FunctionType;

#[macro_use]
mod r#macro;

/// Defines the properties of an intrinsic function that can be called from Mun. These functions
/// are mostly used internally.
pub trait Intrinsic: Sync {
    /// Returns the prototype of the intrinsic
    fn prototype(&self) -> FunctionPrototype;

    /// Returns the IR type for the function
    fn ir_type(&self, context: &Context) -> FunctionType;
}

intrinsics! {
    /// Allocates memory from the runtime to use in code.
    pub fn malloc(size: u64, alignment: u64) -> *mut u8;
}

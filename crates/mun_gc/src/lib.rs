mod handle;
mod mark_sweep;

pub use handle::{GCHandle, RawGCHandle};
pub use mark_sweep::MarkSweep;

/// A trait used by the GC to identify an object.
pub trait Type: Send + Sync {
    /// Returns the size in bytes of an object of this type.
    fn size(&self) -> usize;

    /// Returns the alignment of a type
    fn alignment(&self) -> usize;
}

/// An object that can be used to allocate and collect memory.
pub trait GCRuntime<T: Type>: Send + Sync {
    /// Allocates an object of the given type returning a GCHandle
    fn alloc_object(&self, ty: T) -> GCHandle;

    /// Creates a shallow copy of `obj` and returns a handle to it.
    ///
    /// # Safety
    ///
    /// This method is unsafe because the passed GCHandle could point to random memory.
    unsafe fn clone_object(&self, obj: GCHandle) -> GCHandle;

    /// Returns the type of the specified `obj`.
    ///
    /// # Safety
    ///
    /// This method is unsafe because the passed GCHandle could point to random memory.
    unsafe fn object_type(&self, obj: GCHandle) -> T;

    /// Tell the runtime that the specified object should be considered a root which keeps all other
    /// objects it references alive.
    ///
    /// # Safety
    ///
    /// This method is unsafe because the passed GCHandle could point to random memory.
    unsafe fn set_root(&self, obj: GCHandle, is_root: bool);
}

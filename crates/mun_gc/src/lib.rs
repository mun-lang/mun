mod handle;

pub use handle::{GCHandle, RawGCHandle};

/// A trait used by the GC to identify an object.
pub trait Type {
    /// Returns the size in bytes of an object of this type.
    fn size(&self) -> usize;

    /// Returns the alignment of a type
    fn alignment(&self) -> usize;
}

/// An object that can be used to allocate and collect memory
pub trait GCRuntime<T: Type> {
    /// Allocates an object of the given type returning a GCHandle
    fn alloc_object(&mut self, ty: T) -> GCHandle;

    /// Creates a shallow copy of `obj` and returns a handle to it.
    fn clone_object(&mut self, obj: GCHandle) -> GCHandle;

    /// Returns the type of the specified `obj`.
    fn object_type(&self, obj: GCHandle) -> T;

    /// Tell the runtime that the specified object should be considered a root which keeps all other
    /// objects it references alive.
    fn set_root(&self, obj: GCHandle, is_root: bool);
}

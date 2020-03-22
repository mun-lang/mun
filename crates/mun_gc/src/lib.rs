mod handle;
mod mark_sweep;

pub use handle::{GCHandle, GCRootHandle, HasGCHandlePtr, RawGCHandle};
pub use mark_sweep::MarkSweep;

/// A trait used by the GC to identify an object.
pub trait Type: Send + Sync {
    type Trace: Iterator<Item = GCHandle>;

    /// Returns the size in bytes of an object of this type.
    fn size(&self) -> usize;

    /// Returns the alignment of a type
    fn alignment(&self) -> usize;

    /// Returns an iterator to iterate over all GC objects that are referenced by the given object.
    fn trace(&self, obj: GCHandle) -> Self::Trace;
}

/// An object that can be used to allocate and collect memory.
pub trait GCRuntime<T: Type>: Send + Sync {
    /// Allocates an object of the given type returning a GCHandle
    fn alloc_object(&self, ty: T) -> GCHandle;

    /// Returns the type of the specified `obj`.
    ///
    /// # Safety
    ///
    /// This method is unsafe because the passed GCHandle could point to random memory.
    unsafe fn object_type(&self, obj: GCHandle) -> T;

    /// Tell the runtime that the specified object should be considered a root which keeps all other
    /// objects it references alive. Objects marked as root, must also be unrooted before they can
    /// be collected. Internally this increments a root refcount.
    ///
    /// # Safety
    ///
    /// This method is unsafe because the passed GCHandle could point to random memory.
    unsafe fn root(&self, obj: GCHandle);

    /// Tell the runtime that the specified object should unrooted which keeps all other
    /// objects it references alive. Objects marked as root, must also be unrooted before they can
    /// be collected. Internally this decrements a root refcount. When the refcount reaches 0, the
    /// object is considered non-rooted.
    ///
    /// # Safety
    ///
    /// This method is unsafe because the passed GCHandle could point to random memory.
    unsafe fn unroot(&self, obj: GCHandle);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The GC performed an allocation
    Allocation(GCHandle),

    /// A GC cycle started
    Start,

    /// A deallocation took place
    Deallocation(GCHandle),

    /// A GC cycle ended
    End,
}

pub trait GCObserver: Send + Sync {
    fn event(&self, _event: Event) {}
}

#[derive(Clone, Default)]
pub struct NoopObserver;
impl GCObserver for NoopObserver {}

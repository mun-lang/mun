mod handle;
mod mark_sweep;
mod root_handle;

pub use handle::{GCPtr, HasIndirectionPtr, RawGCPtr};
pub use mark_sweep::MarkSweep;
pub use root_handle::GCRootHandle;

/// Contains stats about the current state of a GC implementation
#[derive(Debug, Clone, Default)]
pub struct Stats {
    pub allocated_memory: usize,
}

/// A trait used by the GC to identify an object.
pub trait Type: Send + Sync {
    type Trace: Iterator<Item = GCPtr>;

    /// Returns the size in bytes of an object of this type.
    fn size(&self) -> usize;

    /// Returns the alignment of a type
    fn alignment(&self) -> usize;

    /// Returns an iterator to iterate over all GC objects that are referenced by the given object.
    fn trace(&self, obj: GCPtr) -> Self::Trace;
}

/// An object that can be used to allocate and collect memory.
pub trait GCRuntime<T: Type>: Send + Sync {
    /// Allocates an object of the given type returning a GCPtr
    fn alloc(&self, ty: T) -> GCPtr;

    /// Returns the type of the specified `obj`.
    ///
    /// # Safety
    ///
    /// This method is unsafe because the passed GCPtr could point to random memory.
    unsafe fn ptr_type(&self, obj: GCPtr) -> T;

    /// Tell the runtime that the specified object should be considered a root which keeps all other
    /// objects it references alive. Objects marked as root, must also be unrooted before they can
    /// be collected. Internally this increments a root refcount.
    ///
    /// # Safety
    ///
    /// This method is unsafe because the passed GCPtr could point to random memory.
    unsafe fn root(&self, obj: GCPtr);

    /// Tell the runtime that the specified object should unrooted which keeps all other
    /// objects it references alive. Objects marked as root, must also be unrooted before they can
    /// be collected. Internally this decrements a root refcount. When the refcount reaches 0, the
    /// object is considered non-rooted.
    ///
    /// # Safety
    ///
    /// This method is unsafe because the passed GCPtr could point to random memory.
    unsafe fn unroot(&self, obj: GCPtr);

    /// Returns stats about the current state of the runtime.
    fn stats(&self) -> Stats;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The GC performed an allocation
    Allocation(GCPtr),

    /// A GC cycle started
    Start,

    /// A deallocation took place
    Deallocation(GCPtr),

    /// A GC cycle ended
    End,
}

/// A `Observer` is trait that can receive `Event`s from a GC implementation. A `GCRuntime` can
/// be typed by a `GCObserver` which enables optional tracing of events.
pub trait Observer: Send + Sync {
    fn event(&self, _event: Event) {}
}

/// A default implementation of a `Observer` which ensures that the compiler does not generate
/// code for event handling.
#[derive(Clone, Default)]
pub struct NoopObserver;
impl Observer for NoopObserver {}

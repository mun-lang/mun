mod array;
mod mark_sweep;
mod ptr;
mod root_ptr;

use crate::r#type::Type;
use std::marker::PhantomData;
use std::ptr::NonNull;

pub use mark_sweep::MarkSweep;
pub use ptr::{GcPtr, HasIndirectionPtr, RawGcPtr};
pub use root_ptr::GcRootPtr;

/// Contains stats about the current state of a GC implementation
#[derive(Debug, Clone, Default)]
pub struct Stats {
    pub allocated_memory: usize,
}

/// A trait used to trace an object type.
pub trait TypeTrace: Send + Sync {
    type Trace: Iterator<Item = GcPtr>;

    /// Returns an iterator to iterate over all GC objects that are referenced by the given object.
    fn trace(&self, obj: GcPtr) -> Self::Trace;
}

/// A trait used to iterate over array elements
pub trait ArrayHandle: Sized {
    type Iterator: Iterator<Item = NonNull<u8>>;

    /// Returns the length of the array
    fn length(&self) -> usize;

    /// Returns the capacity of the array
    fn capacity(&self) -> usize;

    /// Returns an iterator over the elements of the array
    fn elements(&self) -> Self::Iterator;
}

/// An object that can be used to allocate and collect memory.
pub trait GcRuntime: Send + Sync {
    type Array: ArrayHandle;

    /// Allocates an object of the given type returning a GcPtr
    fn alloc(&self, ty: &Type) -> GcPtr;

    /// Allocates an array of the given type. `ty` must be an array type.
    fn alloc_array(&self, ty: &Type, n: usize) -> GcPtr;

    /// Returns the type of the specified `obj`.
    fn ptr_type(&self, obj: GcPtr) -> Type;

    /// Returns array information of the specified handle
    fn array(&self, handle: GcPtr) -> Option<Self::Array>;

    /// Roots the specified `obj`, which keeps it and objects it references alive. Objects marked
    /// as root, must call `unroot` before they can be collected. An object can be rooted multiple
    /// times, but you must make sure to call `unroot` an equal number of times before the object
    /// can be collected.
    fn root(&self, obj: GcPtr);

    /// Unroots the specified `obj`, potentially allowing it and objects it references to be
    /// collected. An object can be rooted multiple times, so you must make sure to call `unroot`
    /// the same number of times as `root` was called before the object can be collected.
    fn unroot(&self, obj: GcPtr);

    /// Returns stats about the current state of the runtime.
    fn stats(&self) -> Stats;
}

/// The `Observer` trait allows receiving of `Event`s.
pub trait Observer: Send + Sync {
    type Event;

    fn event(&self, _event: Self::Event) {}
}

/// An `Event` is an event that can be emitted by a `GcRuntime` through the use of an `Observer`.
/// This enables tracking of the runtimes behavior which is useful for testing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The GC performed an allocation
    Allocation(GcPtr),

    /// A GC cycle started
    Start,

    /// A deallocation took place
    Deallocation(GcPtr),

    /// A GC cycle ended
    End,
}

/// A default implementation of an `Observer` which ensures that the compiler does not generate
/// code for event handling.
#[derive(Clone)]
pub struct NoopObserver<T: Send + Sync> {
    data: PhantomData<T>,
}
impl<T: Send + Sync> Observer for NoopObserver<T> {
    type Event = T;
}
impl<T: Send + Sync> Default for NoopObserver<T> {
    fn default() -> Self {
        NoopObserver { data: PhantomData }
    }
}

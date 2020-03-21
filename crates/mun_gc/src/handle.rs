use crate::{GCRuntime, Type};
use std::marker::PhantomData;
use std::sync::{Arc, Weak};

/// A `GCHandle` is what you interact with outside of the allocator. It is a pointer to a piece of
/// memory that points to the actual data stored in memory.
///
/// This creates an indirection that must be followed to get to the actual data of the object. Note
/// that the indirection pointer must therefor be pinned in memory whereas the pointer stored
/// at the indirection may change.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct GCHandle(RawGCHandle);

/// A `GCHandle` is thread safe.
unsafe impl Send for GCHandle {}
unsafe impl Sync for GCHandle {}

/// A `RawGCHandle` is an unsafe version of a `GCHandle`. It represents the raw internal pointer
/// semantics used by the runtime.
pub type RawGCHandle = *const *mut std::ffi::c_void;

pub trait HasGCHandlePtr {
    /// Returns a pointer to the referenced memory.
    ///
    /// # Safety
    ///
    /// This method is unsafe because casting to a generic type T may be unsafe. We don't know the
    /// type of the stored data.
    unsafe fn get_ptr<T: Sized>(&self) -> *mut T;
}

impl HasGCHandlePtr for GCHandle {
    unsafe fn get_ptr<T: Sized>(&self) -> *mut T {
        (*self.0).cast::<T>()
    }
}

impl Into<RawGCHandle> for GCHandle {
    fn into(self) -> RawGCHandle {
        self.0
    }
}

impl Into<GCHandle> for RawGCHandle {
    fn into(self) -> GCHandle {
        GCHandle(self)
    }
}

/// A `GCHandle` that automatically roots and unroots its internal `GCHandle`.
pub struct GCRootHandle<T: Type, G: GCRuntime<T>> {
    handle: GCHandle,
    runtime: Weak<G>,
    ty: PhantomData<T>,
}

impl<T: Type, G: GCRuntime<T>> Clone for GCRootHandle<T, G> {
    fn clone(&self) -> Self {
        if let Some(runtime) = self.runtime.upgrade() {
            unsafe { runtime.root(self.handle) }
        }
        Self {
            handle: self.handle,
            runtime: self.runtime.clone(),
            ty: Default::default(),
        }
    }
}

impl<T: Type, G: GCRuntime<T>> GCRootHandle<T, G> {
    /// Constructs a new GCRootHandle from a runtime and a handle
    ///
    /// # Safety
    ///
    /// This method is unsafe because the passed GCHandle could point to random memory.
    pub unsafe fn new(runtime: &Arc<G>, handle: GCHandle) -> Self {
        runtime.root(handle);
        Self {
            handle,
            runtime: Arc::downgrade(runtime),
            ty: Default::default(),
        }
    }

    /// Returns the handle of this instance
    pub fn handle(&self) -> GCHandle {
        self.handle
    }
}

impl<T: Type, G: GCRuntime<T>> Into<GCHandle> for GCRootHandle<T, G> {
    fn into(self) -> GCHandle {
        self.handle
    }
}

impl<T: Type, G: GCRuntime<T>> Drop for GCRootHandle<T, G> {
    fn drop(&mut self) {
        if let Some(runtime) = self.runtime.upgrade() {
            unsafe { runtime.unroot(self.handle) }
        }
    }
}

impl<T: Type, G: GCRuntime<T>> HasGCHandlePtr for GCRootHandle<T, G> {
    unsafe fn get_ptr<R: Sized>(&self) -> *mut R {
        self.handle.get_ptr()
    }
}

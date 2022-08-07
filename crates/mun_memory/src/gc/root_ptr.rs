use crate::gc::{GcPtr, GcRuntime, HasIndirectionPtr};
use std::sync::{Arc, Weak};

/// A `GcPtr` that automatically roots and unroots its internal `GcPtr`.
pub struct GcRootPtr<G>
where
    G: GcRuntime,
{
    handle: GcPtr,
    runtime: Weak<G>,
}

impl<G> Clone for GcRootPtr<G>
where
    G: GcRuntime,
{
    fn clone(&self) -> Self {
        if let Some(runtime) = self.runtime.upgrade() {
            runtime.as_ref().root(self.handle)
        }
        Self {
            handle: self.handle,
            runtime: self.runtime.clone(),
        }
    }
}

impl<G> GcRootPtr<G>
where
    G: GcRuntime,
{
    /// Constructs a new GCRootHandle from a runtime and a handle
    pub fn new(runtime: &Arc<G>, handle: GcPtr) -> Self {
        runtime.as_ref().root(handle);
        Self {
            handle,
            runtime: Arc::downgrade(runtime),
        }
    }

    /// Returns the runtime that owns the memory
    pub fn runtime(&self) -> &Weak<G> {
        &self.runtime
    }

    /// Returns the handle of this instance
    pub fn handle(&self) -> GcPtr {
        self.handle
    }

    /// Unroots the handle consuming self and returning the unrooted handle
    /// TODO: Should this simply return nothing, since the returned handle may be collected at any
    ///     time?
    pub fn unroot(self) -> GcPtr {
        self.handle
    }
}

impl<G> From<GcRootPtr<G>> for GcPtr
where
    G: GcRuntime,
{
    fn from(ptr: GcRootPtr<G>) -> Self {
        ptr.handle
    }
}

impl<G> Drop for GcRootPtr<G>
where
    G: GcRuntime,
{
    fn drop(&mut self) {
        if let Some(runtime) = self.runtime.upgrade() {
            runtime.as_ref().unroot(self.handle)
        }
    }
}

impl<G> HasIndirectionPtr for GcRootPtr<G>
where
    G: GcRuntime,
{
    unsafe fn deref<R: Sized>(&self) -> *const R {
        self.handle.deref()
    }
}

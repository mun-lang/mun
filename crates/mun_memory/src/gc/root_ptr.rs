use crate::gc::{GcPtr, GcRuntime, HasIndirectionPtr};
use std::sync::{Arc, Weak};

/// A `GcPtr` that automatically roots and unroots its internal `GcPtr`.
pub struct GcRootPtr<G: GcRuntime> {
    handle: GcPtr,
    runtime: Weak<G>,
}

impl<G: GcRuntime> Clone for GcRootPtr<G> {
    fn clone(&self) -> Self {
        if let Some(runtime) = self.runtime.upgrade() {
            runtime.root(self.handle)
        }
        Self {
            handle: self.handle,
            runtime: self.runtime.clone(),
        }
    }
}

impl<G: GcRuntime> GcRootPtr<G> {
    /// Constructs a new GCRootHandle from a runtime and a handle
    pub fn new(runtime: &Arc<G>, handle: GcPtr) -> Self {
        runtime.root(handle);
        Self {
            handle,
            runtime: Arc::downgrade(runtime),
        }
    }

    /// Returns the handle of this instance
    pub fn handle(&self) -> GcPtr {
        self.handle
    }

    /// Unroots the handle consuming self and returning the unrooted handle
    pub fn unroot(self) -> GcPtr {
        self.handle
    }
}

impl<G: GcRuntime> From<GcRootPtr<G>> for GcPtr {
    fn from(ptr: GcRootPtr<G>) -> Self {
        ptr.handle
    }
}

impl<G: GcRuntime> Drop for GcRootPtr<G> {
    fn drop(&mut self) {
        if let Some(runtime) = self.runtime.upgrade() {
            runtime.unroot(self.handle)
        }
    }
}

impl<G: GcRuntime> HasIndirectionPtr for GcRootPtr<G> {
    unsafe fn deref<R: Sized>(&self) -> *const R {
        self.handle.deref()
    }
}

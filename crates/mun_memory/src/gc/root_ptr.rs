use crate::{
    gc::{GcPtr, GcRuntime, HasIndirectionPtr, TypeTrace},
    TypeLayout,
};
use std::marker::PhantomData;
use std::sync::{Arc, Weak};

/// A `GcPtr` that automatically roots and unroots its internal `GcPtr`.
pub struct GcRootPtr<T: TypeLayout + TypeTrace, G: GcRuntime<T>> {
    handle: GcPtr,
    runtime: Weak<G>,
    ty: PhantomData<T>,
}

impl<T: TypeLayout + TypeTrace, G: GcRuntime<T>> Clone for GcRootPtr<T, G> {
    fn clone(&self) -> Self {
        if let Some(runtime) = self.runtime.upgrade() {
            runtime.root(self.handle)
        }
        Self {
            handle: self.handle,
            runtime: self.runtime.clone(),
            ty: PhantomData,
        }
    }
}

impl<T: TypeLayout + TypeTrace, G: GcRuntime<T>> GcRootPtr<T, G> {
    /// Constructs a new GCRootHandle from a runtime and a handle
    pub fn new(runtime: &Arc<G>, handle: GcPtr) -> Self {
        runtime.root(handle);
        Self {
            handle,
            runtime: Arc::downgrade(runtime),
            ty: PhantomData,
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

impl<T: TypeLayout + TypeTrace, G: GcRuntime<T>> Into<GcPtr> for GcRootPtr<T, G> {
    fn into(self) -> GcPtr {
        self.handle
    }
}

impl<T: TypeLayout + TypeTrace, G: GcRuntime<T>> Drop for GcRootPtr<T, G> {
    fn drop(&mut self) {
        if let Some(runtime) = self.runtime.upgrade() {
            runtime.unroot(self.handle)
        }
    }
}

impl<T: TypeLayout + TypeTrace, G: GcRuntime<T>> HasIndirectionPtr for GcRootPtr<T, G> {
    unsafe fn deref<R: Sized>(&self) -> *const R {
        self.handle.deref()
    }
}

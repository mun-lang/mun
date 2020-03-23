use crate::{GCPtr, GCRuntime, HasIndirectionPtr, Type};
use std::marker::PhantomData;
use std::sync::{Arc, Weak};

/// A `GCPtr` that automatically roots and unroots its internal `GCPtr`.
pub struct GCRootHandle<T: Type, G: GCRuntime<T>> {
    handle: GCPtr,
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
    /// This method is unsafe because the passed GCPtr could point to random memory.
    pub unsafe fn new(runtime: &Arc<G>, handle: GCPtr) -> Self {
        runtime.root(handle);
        Self {
            handle,
            runtime: Arc::downgrade(runtime),
            ty: Default::default(),
        }
    }

    /// Returns the handle of this instance
    pub fn handle(&self) -> GCPtr {
        self.handle
    }

    /// Unroots the handle consuming self and returning the unrooted handle
    pub fn unroot(self) -> GCPtr {
        self.handle
    }
}

impl<T: Type, G: GCRuntime<T>> Into<GCPtr> for GCRootHandle<T, G> {
    fn into(self) -> GCPtr {
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

impl<T: Type, G: GCRuntime<T>> HasIndirectionPtr for GCRootHandle<T, G> {
    unsafe fn deref<R: Sized>(&self) -> *const R {
        self.handle.deref()
    }
}

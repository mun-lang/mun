use std::ptr::NonNull;

/// A `GcPtr` is what you interact with outside of the allocator. It is a pointer to a piece of
/// memory that points to the actual data stored in memory.
///
/// This creates an indirection that must be followed to get to the actual data of the object. Note
/// that the `GcPtr` must therefore be pinned in memory whereas the contained memory pointer may
/// change.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct GcPtr(RawGcPtr);

/// A `GcPtr` is thread safe.
unsafe impl Send for GcPtr {}
unsafe impl Sync for GcPtr {}

/// A `RawGcPtr` is an unsafe version of a `GcPtr`. It represents the raw internal pointer
/// semantics used by the runtime.
pub type RawGcPtr = *const *mut std::ffi::c_void;

pub trait HasIndirectionPtr {
    /// Returns a pointer to the referenced memory.
    ///
    /// # Safety
    ///
    /// This is an unsafe method because derefencing could result in an access violation.
    unsafe fn deref<T: Sized>(&self) -> NonNull<T>;
}

impl HasIndirectionPtr for GcPtr {
    unsafe fn deref<T: Sized>(&self) -> NonNull<T> {
        NonNull::new_unchecked(*self.0 as *mut T)
    }
}

impl From<GcPtr> for RawGcPtr {
    fn from(ptr: GcPtr) -> Self {
        ptr.0
    }
}

impl From<RawGcPtr> for GcPtr {
    fn from(ptr: RawGcPtr) -> Self {
        GcPtr(ptr)
    }
}

impl GcPtr {
    pub(crate) fn as_ptr(self) -> RawGcPtr {
        self.0
    }
}

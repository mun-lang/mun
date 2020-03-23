/// A `GCPtr` is what you interact with outside of the allocator. It is a pointer to a piece of
/// memory that points to the actual data stored in memory.
///
/// This creates an indirection that must be followed to get to the actual data of the object. Note
/// that the indirection pointer must therefor be pinned in memory whereas the pointer stored
/// at the indirection may change.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct GCPtr(RawGCPtr);

/// A `GCPtr` is thread safe.
unsafe impl Send for GCPtr {}
unsafe impl Sync for GCPtr {}

/// A `RawGCPtr` is an unsafe version of a `GCPtr`. It represents the raw internal pointer
/// semantics used by the runtime.
pub type RawGCPtr = *const *mut std::ffi::c_void;

pub trait HasIndirectionPtr {
    /// Returns a pointer to the referenced memory.
    ///
    /// # Safety
    ///
    /// This is an unsafe method because derefencing could result in an access violation.
    unsafe fn deref<T: Sized>(&self) -> *const T;

    /// Returns a mutable pointer to the referenced memory.
    ///
    /// # Safety
    ///
    /// This is an unsafe method because derefencing could result in an access violation.
    unsafe fn deref_mut<T: Sized>(&self) -> *mut T {
        self.deref::<T>() as *mut _
    }
}

impl HasIndirectionPtr for GCPtr {
    unsafe fn deref<T: Sized>(&self) -> *const T {
        (*self.0).cast()
    }
}

impl Into<RawGCPtr> for GCPtr {
    fn into(self) -> RawGCPtr {
        self.0
    }
}

impl Into<GCPtr> for RawGCPtr {
    fn into(self) -> GCPtr {
        GCPtr(self)
    }
}

impl GCPtr {
    pub(crate) fn as_ptr(self) -> RawGCPtr {
        self.0
    }
}

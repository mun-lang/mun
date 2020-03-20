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

impl GCHandle {
    /// Returns a pointer to the referenced memory.
    ///
    /// # Safety
    ///
    /// This method is unsafe because casting to a generic type T may be unsafe. We don't know the
    /// type of the stored data.
    pub unsafe fn get_ptr<T: Sized>(self) -> *mut T {
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

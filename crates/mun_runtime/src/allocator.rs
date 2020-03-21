#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct RawTypeInfo(*const abi::TypeInfo);

impl Into<RawTypeInfo> for *const abi::TypeInfo {
    fn into(self) -> RawTypeInfo {
        RawTypeInfo(self)
    }
}

unsafe impl Send for RawTypeInfo {}
unsafe impl Sync for RawTypeInfo {}

impl gc::Type for RawTypeInfo {
    fn size(&self) -> usize {
        unsafe { (*self.0).size_in_bytes() }
    }

    fn alignment(&self) -> usize {
        unsafe { (*self.0).alignment() }
    }
}

/// Defines an allocator used by the `Runtime`
pub type Allocator = gc::MarkSweep<RawTypeInfo>;

pub use gc::GCHandle;

pub type GCRootHandle = gc::GCRootHandle<RawTypeInfo, Allocator>;

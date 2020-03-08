use std::alloc::Layout;

/// Provides allocator capabilities for a runtime.
#[derive(Debug, Default)]
pub struct Allocator {}

impl Allocator {
    /// Allocates a new instance of an Allocator
    pub fn new() -> Self {
        Default::default()
    }

    /// Allocates a block of memory
    pub(crate) fn alloc(&self, size: u64, alignment: u64) -> *mut u8 {
        unsafe {
            std::alloc::alloc(Layout::from_size_align(size as usize, alignment as usize).unwrap())
        }
    }
}

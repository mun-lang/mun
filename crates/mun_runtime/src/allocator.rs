use gc::{GCHandle, RawGCHandle};
use parking_lot::RwLock;
use std::alloc::Layout;
use std::collections::HashMap;
use std::ops::Deref;
use std::{pin::Pin, ptr};

#[repr(transparent)]
struct RawTypeInfo(*const abi::TypeInfo);

impl gc::Type for RawTypeInfo {
    fn size(&self) -> usize {
        unsafe { (*self.0).size_in_bytes() as usize }
    }

    fn alignment(&self) -> usize {
        unsafe { (*self.0).alignment() as usize }
    }
}

#[derive(Debug)]
#[repr(C)]
struct ObjectInfo {
    pub ptr: *mut u8,
    pub type_info: *const abi::TypeInfo,
}

/// Provides allocator capabilities for a runtime.
#[derive(Debug, Default)]
pub struct Allocator {
    objects: RwLock<HashMap<GCHandle, Pin<Box<ObjectInfo>>>>,
}

impl Allocator {
    /// Allocates a new instance of an Allocator
    pub fn new() -> Self {
        Default::default()
    }

    /// Allocates a block of memory
    fn alloc(&self, size: usize, alignment: usize) -> *mut u8 {
        unsafe { std::alloc::alloc(Layout::from_size_align_unchecked(size, alignment)) }
    }

    /// Allocates a managed object of the specified type.
    ///
    /// # Safety
    ///
    /// `type_info` must be a valid pointer and remain valid throughout the lifetime of the created
    /// object.
    pub(crate) unsafe fn create_object(&self, type_info: *const abi::TypeInfo) -> GCHandle {
        let type_info = type_info.as_ref().unwrap();

        let ptr = self.alloc(type_info.size_in_bytes(), type_info.alignment());
        let object = Box::pin(ObjectInfo { ptr, type_info });

        // We want to return a pointer to the `ObjectInfo`, to be used as handle.
        let handle = (object.as_ref().deref() as *const _ as RawGCHandle).into();

        let mut objects = self.objects.write();
        objects.insert(handle, object);

        handle
    }

    /// Creates a shallow clone of the `src` object at a newly allocated memory location.
    ///
    /// # Safety
    ///
    /// `src` must be a valid pointer.
    pub(crate) unsafe fn clone_object(&self, src: GCHandle) -> GCHandle {
        let clone = {
            let objects = self.objects.read();
            let src = objects
                .get(&src)
                .unwrap_or_else(|| panic!("Object with handle '{:?}' does not exist.", src));

            let type_info = src.type_info.as_ref().unwrap();

            let size = type_info.size_in_bytes();
            let dest = self.alloc(size, type_info.alignment());
            ptr::copy_nonoverlapping(src.ptr, dest, size as usize);

            Box::pin(ObjectInfo {
                ptr: dest,
                type_info,
            })
        };

        // We want to return a pointer to the `ObjectInfo`, to be used as handle.
        let handle = (clone.as_ref().deref() as *const _ as RawGCHandle).into();

        let mut objects = self.objects.write();
        objects.insert(handle, clone);

        handle
    }
}

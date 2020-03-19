use parking_lot::RwLock;
use std::alloc::Layout;
use std::collections::HashMap;
use std::ops::Deref;
use std::{pin::Pin, ptr};

#[derive(Debug)]
#[repr(C)]
pub struct ObjectInfo {
    pub ptr: *mut u8,
    pub type_info: *const abi::TypeInfo,
}

pub type ObjectHandle = *const ObjectInfo;

/// Provides allocator capabilities for a runtime.
#[derive(Debug, Default)]
pub struct Allocator {
    objects: RwLock<HashMap<ObjectHandle, Pin<Box<ObjectInfo>>>>,
}

impl Allocator {
    /// Allocates a new instance of an Allocator
    pub fn new() -> Self {
        Default::default()
    }

    /// Allocates a block of memory
    fn alloc(&self, size: u64, alignment: u64) -> *mut u8 {
        unsafe {
            std::alloc::alloc(Layout::from_size_align(size as usize, alignment as usize).unwrap())
        }
    }

    /// Allocates a managed object of the specified type.
    ///
    /// # Safety
    ///
    /// `type_info` must be a valid pointer and remain valid throughout the lifetime of the created
    /// object.
    pub(crate) unsafe fn create_object(&self, type_info: *const abi::TypeInfo) -> ObjectHandle {
        let type_info = type_info.as_ref().unwrap();

        let ptr = self.alloc(
            type_info.size_in_bytes() as u64,
            type_info.alignment().into(),
        );
        let object = Box::pin(ObjectInfo { ptr, type_info });

        // We want to return a pointer to the `ObjectInfo`, to be used as handle.
        let handle = object.as_ref().deref() as *const _ as ObjectHandle;

        let mut objects = self.objects.write();
        objects.insert(handle, object);

        handle
    }

    /// Creates a shallow clone of the `src` object at a newly allocated memory location.
    ///
    /// # Safety
    ///
    /// `src` must be a valid pointer.
    pub(crate) unsafe fn clone_object(&self, src: ObjectHandle) -> ObjectHandle {
        let clone = {
            let objects = self.objects.read();
            let src = objects
                .get(&src)
                .unwrap_or_else(|| panic!("Object with handle '{:?}' does not exist.", src));

            let type_info = src.type_info.as_ref().unwrap();

            let size = type_info.size_in_bytes();
            let dest = self.alloc(size as u64, type_info.alignment() as u64);
            ptr::copy_nonoverlapping(src.ptr, dest, size as usize);

            Box::pin(ObjectInfo {
                ptr: dest,
                type_info,
            })
        };

        // We want to return a pointer to the `ObjectInfo`, to be used as handle.
        let handle = clone.as_ref().deref() as *const _ as ObjectHandle;

        let mut objects = self.objects.write();
        objects.insert(handle, clone);

        handle
    }
}

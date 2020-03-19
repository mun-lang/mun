use failure::_core::ffi::c_void;
use parking_lot::RwLock;
use std::alloc::Layout;
use std::collections::HashMap;
use std::ops::Deref;
use std::{pin::Pin, ptr};

#[derive(Debug)]
#[repr(C)]
struct ObjectInfo {
    pub ptr: *mut u8,
    pub type_info: *const abi::TypeInfo,
}

pub type RawGCHandle = *const *mut std::ffi::c_void;

/// A GC handle is what you interact with outside of the allocator. It is a pointer to a piece of
/// memory that points to the actual data stored in memory.
///
/// This creates an indirection that must be followed to get to the actual data of the object. Note
/// that the indirection pointer must therefor be pinned in memory whereas the pointer stored
/// at the indirection may change.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct GCHandle(RawGCHandle);

impl GCHandle {
    /// Returns a pointer to the referenced memory
    pub unsafe fn get_ptr<T: Sized>(self) -> *mut T {
        (*self.0).cast::<T>()
    }
}

impl Into<RawGCHandle> for GCHandle {
    fn into(self) -> *const *mut c_void {
        self.0
    }
}

impl Into<GCHandle> for RawGCHandle {
    fn into(self) -> GCHandle {
        GCHandle(self)
    }
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

use crate::{GCHandle, GCRuntime, RawGCHandle, Type};
use parking_lot::RwLock;
use std::alloc::Layout;
use std::collections::HashMap;
use std::ops::Deref;
use std::pin::Pin;
use std::ptr;

/// Implements a simple mark-sweep type memory collector. Uses a HashMap of
#[derive(Debug)]
pub struct MarkSweep<T: Type + Clone> {
    objects: RwLock<HashMap<GCHandle, Pin<Box<ObjectInfo<T>>>>>,
}

impl<T: Type + Clone> Default for MarkSweep<T> {
    fn default() -> Self {
        MarkSweep {
            objects: RwLock::new(HashMap::new()),
        }
    }
}

impl<T: Type + Clone> MarkSweep<T> {
    pub fn new() -> Self {
        Default::default()
    }

    /// Allocates a block of memory
    pub(crate) fn alloc(&self, size: usize, alignment: usize) -> *mut u8 {
        unsafe { std::alloc::alloc(Layout::from_size_align_unchecked(size, alignment)) }
    }
}

impl<T: Type + Clone> GCRuntime<T> for MarkSweep<T> {
    fn alloc_object(&self, ty: T) -> GCHandle {
        let ptr = self.alloc(ty.size(), ty.alignment());
        let object = Box::pin(ObjectInfo { ptr, ty });

        // We want to return a pointer to the `ObjectInfo`, to be used as handle.
        let handle = (object.as_ref().deref() as *const _ as RawGCHandle).into();

        let mut objects = self.objects.write();
        objects.insert(handle, object);

        handle
    }

    unsafe fn clone_object(&self, obj: GCHandle) -> GCHandle {
        let clone = {
            let objects = self.objects.read();
            let src = objects
                .get(&obj)
                .unwrap_or_else(|| panic!("Object with handle '{:?}' does not exist.", obj));

            let size = src.ty.size();
            let dest = self.alloc(src.ty.size(), src.ty.alignment());
            ptr::copy_nonoverlapping(src.ptr, dest, size as usize);

            Box::pin(ObjectInfo {
                ptr: dest,
                ty: src.ty.clone(),
            })
        };

        // We want to return a pointer to the `ObjectInfo`, to be used as handle.
        let handle = (clone.as_ref().deref() as *const _ as RawGCHandle).into();

        let mut objects = self.objects.write();
        objects.insert(handle, clone);

        handle
    }

    unsafe fn object_type(&self, obj: GCHandle) -> T {
        let objects = self.objects.read();
        let src = objects
            .get(&obj)
            .unwrap_or_else(|| panic!("Object with handle '{:?}' does not exist.", obj));

        src.ty.clone()
    }

    unsafe fn root(&self, _obj: GCHandle) {
        // NOOP
    }

    unsafe fn unroot(&self, _obj: GCHandle) {
        // NOOP
    }
}

/// An indirection table that stores the address to the actual memory and the type of the object
#[derive(Debug)]
#[repr(C)]
struct ObjectInfo<T: Type + Clone> {
    pub ptr: *mut u8,
    pub ty: T,
}

/// An `ObjectInfo` is thread-safe.
unsafe impl<T: Type + Clone> Send for ObjectInfo<T> {}
unsafe impl<T: Type + Clone> Sync for ObjectInfo<T> {}

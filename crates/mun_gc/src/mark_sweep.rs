use crate::{Event, GCPtr, GCRuntime, Observer, RawGCPtr, Stats, Type};
use parking_lot::RwLock;
use std::alloc::Layout;
use std::collections::{HashMap, VecDeque};
use std::ops::Deref;
use std::pin::Pin;

/// Implements a simple mark-sweep type memory collector. Uses a HashMap of
#[derive(Debug)]
pub struct MarkSweep<T: Type + Clone, O: Observer> {
    objects: RwLock<HashMap<GCPtr, Pin<Box<ObjectInfo<T>>>>>,
    observer: O,
    stats: RwLock<Stats>,
}

impl<T: Type + Clone, O: Observer + Default> Default for MarkSweep<T, O> {
    fn default() -> Self {
        MarkSweep {
            objects: RwLock::new(HashMap::new()),
            observer: O::default(),
            stats: RwLock::new(Stats::default()),
        }
    }
}

impl<T: Type + Clone, O: Observer + Default> MarkSweep<T, O> {
    pub fn new() -> Self {
        Default::default()
    }
}

impl<T: Type + Clone, O: Observer> MarkSweep<T, O> {
    pub fn with_observer(observer: O) -> Self {
        Self {
            objects: RwLock::new(HashMap::new()),
            observer,
            stats: RwLock::new(Stats::default()),
        }
    }
}

impl<T: Type + Clone, O: Observer> MarkSweep<T, O> {
    /// Allocates a block of memory
    fn alloc_memory(&self, size: usize, alignment: usize) -> *mut u8 {
        unsafe { std::alloc::alloc(Layout::from_size_align_unchecked(size, alignment)) }
    }

    /// Returns the observer
    pub fn observer(&self) -> &O {
        &self.observer
    }
}

impl<T: Type + Clone, O: Observer> GCRuntime<T> for MarkSweep<T, O> {
    fn alloc(&self, ty: T) -> GCPtr {
        let size = ty.size();
        let ptr = self.alloc_memory(ty.size(), ty.alignment());
        let object = Box::pin(ObjectInfo {
            ptr,
            ty,
            roots: 0,
            color: Color::White,
        });

        // We want to return a pointer to the `ObjectInfo`, to be used as handle.
        let handle = (object.as_ref().deref() as *const _ as RawGCPtr).into();

        {
            let mut objects = self.objects.write();
            objects.insert(handle, object);
        }

        {
            let mut stats = self.stats.write();
            stats.allocated_memory += size;
        }

        self.observer.event(Event::Allocation(handle));
        handle
    }

    unsafe fn ptr_type(&self, handle: GCPtr) -> T {
        let _ = self.objects.read();

        // Convert the handle to our internal representation
        let object_info: *const ObjectInfo<T> = handle.into();

        // Return the type of the object
        (*object_info).ty.clone()
    }

    unsafe fn root(&self, handle: GCPtr) {
        let _ = self.objects.write();

        // Convert the handle to our internal representation
        let object_info: *mut ObjectInfo<T> = handle.into();

        // Return the type of the object
        (*object_info).roots += 1;
    }

    unsafe fn unroot(&self, handle: GCPtr) {
        let _ = self.objects.write();

        // Convert the handle to our internal representation
        let object_info: *mut ObjectInfo<T> = handle.into();

        // Return the type of the object
        (*object_info).roots -= 1;
    }

    fn stats(&self) -> Stats {
        self.stats.read().clone()
    }
}

impl<T: Type + Clone, O: Observer + Default> MarkSweep<T, O> {
    /// Collects all memory that is no longer referenced by rooted objects. Returns `true` if memory
    /// was reclaimed, `false` otherwise.
    pub fn collect(&self) -> bool {
        self.observer.event(Event::Start);

        let mut writer = self.objects.write();

        // Get all roots
        let mut roots = writer
            .iter()
            .filter_map(|(_, obj)| {
                if obj.roots > 0 {
                    Some(obj.as_ref().get_ref() as *const _ as *mut ObjectInfo<T>)
                } else {
                    None
                }
            })
            .collect::<VecDeque<_>>();

        // Iterate over all roots
        while let Some(next) = roots.pop_front() {
            let handle = (next as *const _ as RawGCPtr).into();

            // Trace all other objects
            for reference in unsafe { (*next).ty.trace(handle) } {
                let ref_ptr = writer.get_mut(&reference).expect("found invalid reference");
                if ref_ptr.color == Color::White {
                    let ptr = ref_ptr.as_ref().get_ref() as *const _ as *mut ObjectInfo<T>;
                    unsafe { (*ptr).color = Color::Gray };
                    roots.push_back(ptr);
                }
            }

            // This object has been traced
            unsafe {
                (*next).color = Color::Black;
            }
        }

        // Sweep all non-reachable objects
        let size_before = writer.len();
        writer.retain(|h, obj| {
            if obj.color == Color::Black {
                unsafe {
                    obj.as_mut().get_unchecked_mut().color = Color::White;
                }
                true
            } else {
                self.observer.event(Event::Deallocation(*h));
                {
                    let mut stats = self.stats.write();
                    stats.allocated_memory -= obj.ty.size();
                }
                false
            }
        });
        let size_after = writer.len();

        self.observer.event(Event::End);

        size_before != size_after
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Color {
    White,
    Gray,
    Black,
}

/// An indirection table that stores the address to the actual memory and the type of the object
#[derive(Debug)]
#[repr(C)]
struct ObjectInfo<T: Type + Clone> {
    pub ptr: *mut u8,
    pub roots: u32,
    pub color: Color,
    pub ty: T,
}

/// An `ObjectInfo` is thread-safe.
unsafe impl<T: Type + Clone> Send for ObjectInfo<T> {}
unsafe impl<T: Type + Clone> Sync for ObjectInfo<T> {}

impl<T: Type + Clone> Into<*const ObjectInfo<T>> for GCPtr {
    fn into(self) -> *const ObjectInfo<T> {
        self.as_ptr() as *const ObjectInfo<T>
    }
}

impl<T: Type + Clone> Into<*mut ObjectInfo<T>> for GCPtr {
    fn into(self) -> *mut ObjectInfo<T> {
        self.as_ptr() as *mut ObjectInfo<T>
    }
}

impl<T: Type + Clone> Into<GCPtr> for *const ObjectInfo<T> {
    fn into(self) -> GCPtr {
        (self as RawGCPtr).into()
    }
}

impl<T: Type + Clone> Into<GCPtr> for *mut ObjectInfo<T> {
    fn into(self) -> GCPtr {
        (self as RawGCPtr).into()
    }
}

use crate::{Event, GCHandle, GCObserver, GCRuntime, RawGCHandle, Type};
use parking_lot::RwLock;
use std::alloc::Layout;
use std::collections::{HashMap, VecDeque};
use std::ops::Deref;
use std::pin::Pin;

/// Implements a simple mark-sweep type memory collector. Uses a HashMap of
#[derive(Debug)]
pub struct MarkSweep<T: Type + Clone, O: GCObserver> {
    objects: RwLock<HashMap<GCHandle, Pin<Box<ObjectInfo<T>>>>>,
    observer: O,
}

impl<T: Type + Clone, O: GCObserver + Default> Default for MarkSweep<T, O> {
    fn default() -> Self {
        MarkSweep {
            objects: RwLock::new(HashMap::new()),
            observer: O::default(),
        }
    }
}

impl<T: Type + Clone, O: GCObserver + Default> MarkSweep<T, O> {
    pub fn new() -> Self {
        Default::default()
    }
}

impl<T: Type + Clone, O: GCObserver> MarkSweep<T, O> {
    pub fn with_observer(observer: O) -> Self {
        Self {
            objects: RwLock::new(HashMap::new()),
            observer,
        }
    }
}

impl<T: Type + Clone, O: GCObserver> MarkSweep<T, O> {
    /// Allocates a block of memory
    pub(crate) fn alloc(&self, size: usize, alignment: usize) -> *mut u8 {
        unsafe { std::alloc::alloc(Layout::from_size_align_unchecked(size, alignment)) }
    }

    /// Returns the observer
    pub fn observer(&self) -> &O {
        &self.observer
    }
}

impl<T: Type + Clone, O: GCObserver> GCRuntime<T> for MarkSweep<T, O> {
    fn alloc_object(&self, ty: T) -> GCHandle {
        let ptr = self.alloc(ty.size(), ty.alignment());
        let object = Box::pin(ObjectInfo {
            ptr,
            ty,
            roots: 0,
            color: Color::White,
        });

        // We want to return a pointer to the `ObjectInfo`, to be used as handle.
        let handle = (object.as_ref().deref() as *const _ as RawGCHandle).into();

        {
            let mut objects = self.objects.write();
            objects.insert(handle, object);
        }

        self.observer.event(Event::Allocation(handle));
        handle
    }

    unsafe fn object_type(&self, obj: GCHandle) -> T {
        let objects = self.objects.read();
        let src = objects
            .get(&obj)
            .unwrap_or_else(|| panic!("Object with handle '{:?}' does not exist.", obj));

        src.ty.clone()
    }

    unsafe fn root(&self, obj: GCHandle) {
        let mut objects = self.objects.write();
        let src = objects
            .get_mut(&obj)
            .unwrap_or_else(|| panic!("Object with handle '{:?}' does not exist.", obj));
        src.as_mut().get_unchecked_mut().roots += 1;
    }

    unsafe fn unroot(&self, obj: GCHandle) {
        let mut objects = self.objects.write();
        let src = objects
            .get_mut(&obj)
            .unwrap_or_else(|| panic!("Object with handle '{:?}' does not exist.", obj));
        src.as_mut().get_unchecked_mut().roots -= 1;
    }
}

impl<T: Type + Clone, O: GCObserver + Default> MarkSweep<T, O> {
    pub fn collect(&self) {
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
            let handle = (next as *const _ as RawGCHandle).into();

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
        writer.retain(|h, obj| {
            if obj.color == Color::Black {
                unsafe {
                    obj.as_mut().get_unchecked_mut().color = Color::White;
                }
                true
            } else {
                self.observer.event(Event::Deallocation(*h));
                false
            }
        });

        self.observer.event(Event::End);
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

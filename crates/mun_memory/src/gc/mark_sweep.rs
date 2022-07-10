use crate::gc::array::ArrayHeader;
use crate::{
    cast,
    gc::{Event, GcPtr, GcRuntime, HasIndirectionPtr, Observer, RawGcPtr, Stats, TypeTrace},
    mapping::{self, FieldMapping, MemoryMapper},
    type_info::{TypeInfo, TypeInfoData},
};
use mapping::{Conversion, Mapping};
use parking_lot::{RwLock, RwLockReadGuard};
use std::alloc::{Layout, LayoutError};
use std::{
    collections::{HashMap, VecDeque},
    ops::Deref,
    pin::Pin,
    ptr::NonNull,
    sync::Arc,
};

pub struct Trace {
    obj: GcPtr,
    ty: Arc<TypeInfo>,
    index: usize,
}

impl Iterator for Trace {
    type Item = GcPtr;

    fn next(&mut self) -> Option<Self::Item> {
        let struct_ty = self.ty.as_ref().as_struct()?;
        let field_count = struct_ty.fields.len();
        while self.index < field_count {
            let index = self.index;
            self.index += 1;

            let field = &struct_ty.fields[index];
            if let TypeInfoData::Struct(field_struct_ty) = &field.type_info.data {
                if field_struct_ty.memory_kind == abi::StructMemoryKind::Gc {
                    return Some(unsafe {
                        *self
                            .obj
                            .deref::<u8>()
                            .add(usize::from(field.offset))
                            .cast::<GcPtr>()
                    });
                }
            }
        }
        None
    }
}

impl TypeTrace for Arc<TypeInfo> {
    type Trace = Trace;

    fn trace(&self, obj: GcPtr) -> Self::Trace {
        Trace {
            ty: self.clone(),
            obj,
            index: 0,
        }
    }
}

/// Implements a simple mark-sweep type garbage collector.
pub struct MarkSweep<O>
where
    O: Observer<Event = Event>,
{
    objects: RwLock<HashMap<GcPtr, Pin<Box<ObjectInfo>>>>,
    observer: O,
    stats: RwLock<Stats>,
}

impl<O> Default for MarkSweep<O>
where
    O: Observer<Event = Event> + Default,
{
    fn default() -> Self {
        MarkSweep {
            objects: RwLock::new(HashMap::new()),
            observer: O::default(),
            stats: RwLock::new(Stats::default()),
        }
    }
}

impl<O> MarkSweep<O>
where
    O: Observer<Event = Event>,
{
    /// Creates a `MarkSweep` memory collector with the specified `Observer`.
    pub fn with_observer(observer: O) -> Self {
        Self {
            objects: RwLock::new(HashMap::new()),
            observer,
            stats: RwLock::new(Stats::default()),
        }
    }

    /// Logs an allocation
    fn log_alloc(&self, handle: GcPtr, size: usize) {
        {
            let mut stats = self.stats.write();
            stats.allocated_memory += size;
        }

        self.observer.event(Event::Allocation(handle));
    }

    /// Returns the observer
    pub fn observer(&self) -> &O {
        &self.observer
    }
}

fn alloc_obj(ty: Arc<TypeInfo>) -> Pin<Box<ObjectInfo>> {
    let ptr = unsafe { std::alloc::alloc(ty.layout) };
    Box::pin(ObjectInfo {
        data: ObjectInfoData { ptr: ptr.cast() },
        ty,
        roots: 0,
        color: Color::White,
    })
}

/// An error that might occur when requesting memory layout of a type
#[derive(Debug)]
pub enum MemoryLayoutError {
    /// An error that is returned when the memory requested is to large to deal with.
    OutOfBounds,

    /// An error that is returned by constructing a Layout
    LayoutError(LayoutError),
}

impl From<LayoutError> for MemoryLayoutError {
    fn from(err: LayoutError) -> Self {
        MemoryLayoutError::LayoutError(err)
    }
}

pub struct ArrayHandle<'gc> {
    obj: *mut ObjectInfo,
    _lock: RwLockReadGuard<'gc, HashMap<GcPtr, Pin<Box<ObjectInfo>>>>,
}

pub struct ArrayHandleIter {
    remaining: usize,
    next: NonNull<u8>,
    stride: usize,
}

/// Helper object to work with GcPtr that represents an array.
///
/// Arrays are stored in memory with a header which holds the length and capacity. The memory layout
/// of an array looks like this in memory:
///
/// ```text
/// object.data.array ───►┌──────────────┐
///                       │ ArrayHeader  │
///                       └─┬────────────┘
///                         │ padding to align elements
///                       ┌─┴────────────┐
///                       │ element #1   │
///                       └──────────────┘
///                        :
///                       ┌──────────────┐
///                       │ element #n   │
///                       └──────────────┘
/// ```
impl<'gc> ArrayHandle<'gc> {
    /// Returns the type of the stored element.
    pub fn element_type(&self) -> &Arc<TypeInfo> {
        unsafe {
            &(*self.obj)
                .ty
                .as_array()
                .expect("unable to determine element_type, type is not an array")
                .element_ty
        }
    }

    /// Returns a reference to the header
    pub fn header(&self) -> &ArrayHeader {
        // Safety: Safe because at the moment we have a reference to the object which cannot be
        // modified. Also we can be sure this is an array at this point.
        unsafe { &*(*self.obj).data.array }
    }

    /// Sets the length of the array.
    ///
    /// # Safety
    ///
    /// This function is unsafe because the array elements might be left uninitialized.
    pub unsafe fn set_length(&mut self, length: usize) {
        let header = &mut *(*self.obj).data.array;
        debug_assert!(header.capacity >= length);
        header.length = length;
    }

    /// Returns the layout of an element stored in the array.
    ///
    /// Note that this may be different from the layout of the [`Self::element_type`]. If the
    /// element type is a garbage collected type, the array stores references instead of raw
    /// elements.
    pub fn element_layout(&self) -> Layout {
        let element_ty = self.element_type();
        if element_ty.is_stack_allocated() {
            element_ty.layout
        } else {
            Layout::new::<GcPtr>()
        }
    }

    /// Returns the stride in bytes between elements.
    ///
    /// The stride is determined by the size of [`Self::element_layout`] padded to alignment of
    /// layout.
    pub fn element_stride(&self) -> usize {
        self.element_layout().pad_to_align().size()
    }

    /// Returns a pointer to the data.
    pub fn data(&self) -> NonNull<u8> {
        // Determine the offset of the data relative from the start of the array pointer. This
        // the header and the extra alignment padding between the header and the data.
        let element_layout = self.element_layout();
        let header_layout = Layout::new::<ArrayHeader>();
        let (_, padded_header_size) = header_layout
            .extend(element_layout)
            .expect("error creating combined layout of header and element");

        unsafe {
            NonNull::new_unchecked(((*self.obj).data.array as *mut u8).add(padded_header_size))
        }
    }
}

impl<'gc> crate::gc::ArrayHandle for ArrayHandle<'gc> {
    type Iterator = ArrayHandleIter;

    fn length(&self) -> usize {
        self.header().length
    }

    fn capacity(&self) -> usize {
        self.header().capacity
    }

    fn elements(&self) -> Self::Iterator {
        let length = self.length();
        let next = self.data();
        let element_ty = self.element_type();
        let element_layout = if element_ty.is_stack_allocated() {
            element_ty.layout
        } else {
            Layout::new::<GcPtr>()
        };
        ArrayHandleIter {
            remaining: length,
            next,
            stride: element_layout.pad_to_align().size(),
        }
    }
}

impl Iterator for ArrayHandleIter {
    type Item = NonNull<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining > 0 {
            let element = self.next;
            self.remaining -= 1;
            self.next = unsafe { NonNull::new_unchecked(self.next.as_ptr().add(self.stride)) };
            Some(element)
        } else {
            None
        }
    }
}

/// Creates a layout describing the record for `n` instances of `layout`, with a suitable amount of
/// padding between each to ensure that each instance is given its requested size an alignment.
///
/// Implementation taken from `Layout::repeat` (which is currently unstable)
fn repeat_layout(layout: Layout, n: usize) -> Result<Layout, MemoryLayoutError> {
    let len_rounded_up = layout.size().wrapping_add(layout.align()).wrapping_sub(1)
        & !layout.align().wrapping_sub(1);
    let padded_size = layout.size() + len_rounded_up.wrapping_sub(layout.align());
    let alloc_size = padded_size
        .checked_mul(n)
        .ok_or(MemoryLayoutError::OutOfBounds)?;
    Layout::from_size_align(alloc_size, layout.align()).map_err(Into::into)
}

/// Allocates memory for an array type with `length` elements. `array_ty` must be an array type.
fn alloc_array(ty: Arc<TypeInfo>, length: usize) -> Pin<Box<ObjectInfo>> {
    let array_ty = ty
        .as_array()
        .expect("array type doesnt have an element type");

    // Allocate memory for the array data
    let header_layout = Layout::new::<ArrayHeader>();
    let element_ty_layout = if array_ty.element_ty.is_stack_allocated() {
        array_ty.element_ty.layout
    } else {
        Layout::new::<GcPtr>()
    };
    let elements_layout = repeat_layout(element_ty_layout, length)
        .expect("unable to create a memory layout for array elemets");
    let (layout, _) = header_layout
        .extend(elements_layout)
        .expect("unable to create memory layout for array");

    let mut ptr: NonNull<ArrayHeader> = NonNull::new(unsafe { std::alloc::alloc(layout).cast() })
        .expect("error allocating memory for array");
    let array = unsafe { ptr.as_mut() };
    array.length = length;
    array.capacity = length;

    Box::pin(ObjectInfo {
        data: ObjectInfoData {
            array: ptr.as_ptr(),
        },
        ty,
        roots: 0,
        color: Color::White,
    })
}

impl<'a, O> GcRuntime for &'a MarkSweep<O>
where
    O: Observer<Event = Event>,
{
    type Array = ArrayHandle<'a>;

    fn alloc(&self, ty: &Arc<TypeInfo>) -> GcPtr {
        let object = alloc_obj(ty.clone());
        let size = object.layout().size();

        // We want to return a pointer to the `ObjectInfo`, to be used as handle.
        let handle = (object.as_ref().deref() as *const _ as RawGcPtr).into();

        {
            let mut objects = self.objects.write();
            objects.insert(handle, object);
        }

        self.log_alloc(handle, size);
        handle
    }

    fn alloc_array(&self, ty: &Arc<TypeInfo>, n: usize) -> GcPtr {
        let object = alloc_array(ty.clone(), n);
        let size = object.layout().size();

        // We want to return a pointer to the `ObjectInfo`, to be used as handle.
        let handle = (object.as_ref().deref() as *const _ as RawGcPtr).into();

        {
            let mut objects = self.objects.write();
            objects.insert(handle, object);
        }

        self.log_alloc(handle, size);
        handle
    }

    fn ptr_type(&self, handle: GcPtr) -> Arc<TypeInfo> {
        let _lock = self.objects.read();

        // Convert the handle to our internal representation
        let object_info: *const ObjectInfo = handle.into();

        // Return the type of the object
        unsafe { (*object_info).ty.clone() }
    }

    fn array(&self, handle: GcPtr) -> Option<Self::Array> {
        let lock = self.objects.read();
        let obj: *mut ObjectInfo = handle.into();
        unsafe {
            if !(*obj).ty.is_array() {
                return None;
            }
        }

        Some(ArrayHandle { obj, _lock: lock })
    }

    fn root(&self, handle: GcPtr) {
        let _lock = self.objects.write();

        // Convert the handle to our internal representation
        let object_info: *mut ObjectInfo = handle.into();

        unsafe { (*object_info).roots += 1 };
    }

    fn unroot(&self, handle: GcPtr) {
        let _lock = self.objects.write();

        // Convert the handle to our internal representation
        let object_info: *mut ObjectInfo = handle.into();

        unsafe { (*object_info).roots -= 1 };
    }

    fn stats(&self) -> Stats {
        self.stats.read().clone()
    }
}

impl<O> MarkSweep<O>
where
    O: Observer<Event = Event>,
{
    /// Collects all memory that is no longer referenced by rooted objects. Returns `true` if memory
    /// was reclaimed, `false` otherwise.
    pub fn collect(&self) -> bool {
        self.observer.event(Event::Start);

        let mut objects = self.objects.write();

        // Get all roots
        let mut roots = objects
            .iter()
            .filter_map(|(_, obj)| {
                if obj.roots > 0 {
                    Some(obj.as_ref().get_ref() as *const _ as *mut ObjectInfo)
                } else {
                    None
                }
            })
            .collect::<VecDeque<_>>();

        // Iterate over all roots
        while let Some(next) = roots.pop_front() {
            let handle = (next as *const _ as RawGcPtr).into();

            // Trace all other objects
            for reference in unsafe { (*next).ty.trace(handle) } {
                let ref_ptr = objects
                    .get_mut(&reference)
                    .expect("found invalid reference");
                if ref_ptr.color == Color::White {
                    let ptr = ref_ptr.as_ref().get_ref() as *const _ as *mut ObjectInfo;
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
        let size_before = objects.len();
        objects.retain(|h, obj| {
            if obj.color == Color::Black {
                unsafe {
                    obj.as_mut().get_unchecked_mut().color = Color::White;
                }
                true
            } else {
                let value_memory_layout = obj.layout();
                unsafe { std::alloc::dealloc(obj.data.ptr, value_memory_layout) };
                self.observer.event(Event::Deallocation(*h));
                {
                    dbg!(("dealloc", &value_memory_layout));
                    let mut stats = self.stats.write();
                    stats.allocated_memory -= value_memory_layout.size();
                }
                false
            }
        });
        let size_after = objects.len();

        self.observer.event(Event::End);

        size_before != size_after
    }
}

impl<O> MemoryMapper for MarkSweep<O>
where
    O: Observer<Event = Event>,
{
    fn map_memory(&self, mapping: Mapping) -> Vec<GcPtr> {
        let mut objects = self.objects.write();

        // Determine which types are still allocated with deleted types
        let deleted = objects
            .iter()
            .filter_map(|(ptr, object_info)| {
                if mapping.deletions.contains(&object_info.ty) {
                    Some(*ptr)
                } else {
                    None
                }
            })
            .collect();

        // Update type pointers of types that didn't change
        for (old_ty, new_ty) in mapping.identical {
            for object_info in objects.values_mut() {
                if object_info.ty == old_ty {
                    object_info.set(ObjectInfo {
                        data: ObjectInfoData {
                            ptr: unsafe { object_info.data.ptr },
                        },
                        roots: object_info.roots,
                        color: object_info.color,
                        ty: new_ty.clone(),
                    });
                }
            }
        }

        let mut new_allocations = Vec::new();

        for (old_ty, conversion) in mapping.conversions.iter() {
            for object_info in objects.values_mut() {
                if object_info.ty == *old_ty {
                    let src = unsafe { NonNull::new_unchecked(object_info.data.ptr) };
                    let dest = unsafe {
                        NonNull::new_unchecked(std::alloc::alloc_zeroed(conversion.new_ty.layout))
                    };

                    map_fields(
                        self,
                        &mut new_allocations,
                        &mapping.conversions,
                        &conversion.field_mapping,
                        src,
                        dest,
                    );

                    unsafe { std::alloc::dealloc(src.as_ptr(), old_ty.layout) };

                    object_info.set(ObjectInfo {
                        data: ObjectInfoData { ptr: dest.as_ptr() },
                        roots: object_info.roots,
                        color: object_info.color,
                        ty: conversion.new_ty.clone(),
                    });
                }
            }
        }

        // Retroactively store newly allocated objects
        // This cannot be done while mapping because we hold a mutable reference to objects
        for object in new_allocations {
            let size = object.layout().size();
            // We want to return a pointer to the `ObjectInfo`, to
            // be used as handle.
            let handle = (object.as_ref().deref() as *const _ as RawGcPtr).into();
            objects.insert(handle, object);

            self.log_alloc(handle, size);
        }

        return deleted;

        #[allow(clippy::mutable_key_type)]
        fn map_fields<O>(
            gc: &MarkSweep<O>,
            new_allocations: &mut Vec<Pin<Box<ObjectInfo>>>,
            conversions: &HashMap<Arc<TypeInfo>, Conversion>,
            mapping: &[FieldMapping],
            src: NonNull<u8>,
            dest: NonNull<u8>,
        ) where
            O: Observer<Event = Event>,
        {
            for FieldMapping {
                new_ty,
                new_offset,
                action,
            } in mapping.iter()
            {
                let field_dest = {
                    let mut dest = dest.as_ptr() as usize;
                    dest += new_offset;
                    dest as *mut u8
                };

                match action {
                    mapping::Action::Cast { old_offset, old_ty } => {
                        let field_src = {
                            let mut src = src.as_ptr() as usize;
                            src += old_offset;
                            src as *mut u8
                        };

                        if old_ty.data.is_struct() {
                            debug_assert!(new_ty.data.is_struct());

                            // When the name is the same, we are dealing with the same struct,
                            // but different internals
                            let is_same_struct = old_ty.name == new_ty.name;

                            // If the same struct changed, there must also be a conversion
                            let conversion = conversions.get(old_ty);

                            if old_ty.is_stack_allocated() {
                                if new_ty.is_stack_allocated() {
                                    // struct(value) -> struct(value)
                                    if is_same_struct {
                                        // Map in-memory struct to in-memory struct
                                        map_fields(
                                            gc,
                                            new_allocations,
                                            conversions,
                                            &conversion.as_ref().unwrap().field_mapping,
                                            unsafe { NonNull::new_unchecked(field_src) },
                                            unsafe { NonNull::new_unchecked(field_dest) },
                                        );
                                    } else {
                                        // Use previously zero-initialized memory
                                    }
                                } else {
                                    // struct(value) -> struct(gc)
                                    let object = alloc_obj(new_ty.clone());

                                    // We want to return a pointer to the `ObjectInfo`, to be used as handle.
                                    let handle =
                                        (object.as_ref().deref() as *const _ as RawGcPtr).into();

                                    if is_same_struct {
                                        // Map in-memory struct to heap-allocated struct
                                        map_fields(
                                            gc,
                                            new_allocations,
                                            conversions,
                                            &conversion.as_ref().unwrap().field_mapping,
                                            unsafe { NonNull::new_unchecked(field_src) },
                                            unsafe { NonNull::new_unchecked(field_dest) },
                                        );
                                    } else {
                                        // Zero initialize heap-allocated object
                                        unsafe {
                                            std::ptr::write_bytes(
                                                (*object).data.ptr,
                                                0,
                                                new_ty.layout.size(),
                                            )
                                        };
                                    }

                                    // Write handle to field
                                    let field_handle = field_dest.cast::<GcPtr>();
                                    unsafe { *field_handle = handle };

                                    new_allocations.push(object);
                                }
                            } else if !new_ty.is_stack_allocated() {
                                // struct(gc) -> struct(gc)
                                let field_src = field_src.cast::<GcPtr>();
                                let field_dest = field_dest.cast::<GcPtr>();

                                if is_same_struct {
                                    // Only copy the `GcPtr`. Memory will already be mapped.
                                    unsafe {
                                        *field_dest = *field_src;
                                    }
                                } else {
                                    let object = alloc_obj(new_ty.clone());

                                    // We want to return a pointer to the `ObjectInfo`, to
                                    // be used as handle.
                                    let handle =
                                        (object.as_ref().deref() as *const _ as RawGcPtr).into();

                                    // Zero-initialize heap-allocated object
                                    unsafe {
                                        std::ptr::write_bytes(
                                            object.data.ptr,
                                            0,
                                            new_ty.layout.size(),
                                        )
                                    };

                                    // Write handle to field
                                    unsafe {
                                        *field_dest = handle;
                                    }

                                    new_allocations.push(object);
                                }
                            } else {
                                // struct(gc) -> struct(value)
                                let field_handle = unsafe { *field_src.cast::<GcPtr>() };

                                // Convert the handle to our internal representation
                                // Safety: we already hold a write lock on `objects`, so
                                // this is legal.
                                let obj: *mut ObjectInfo = field_handle.into();
                                let obj = unsafe { &*obj };

                                if is_same_struct {
                                    if obj.ty == *old_ty {
                                        // The object still needs to be mapped
                                        // Map heap-allocated struct to in-memory struct
                                        map_fields(
                                            gc,
                                            new_allocations,
                                            conversions,
                                            &conversion.as_ref().unwrap().field_mapping,
                                            unsafe { NonNull::new_unchecked(obj.data.ptr) },
                                            unsafe { NonNull::new_unchecked(field_dest) },
                                        );
                                    } else {
                                        // The object was already mapped
                                        debug_assert!(obj.ty == *new_ty);

                                        // Copy from heap-allocated struct to in-memory struct
                                        unsafe {
                                            std::ptr::copy_nonoverlapping(
                                                obj.data.ptr,
                                                field_dest,
                                                obj.ty.layout.size(),
                                            )
                                        };
                                    }
                                } else {
                                    // Use previously zero-initialized memory
                                }
                            }
                        } else if !cast::try_cast_from_to(
                            old_ty.clone(),
                            new_ty.clone(),
                            unsafe { NonNull::new_unchecked(field_src) },
                            unsafe { NonNull::new_unchecked(field_dest) },
                        ) {
                            // Failed to cast. Use the previously zero-initialized value instead
                        }
                    }
                    mapping::Action::Copy { old_offset } => {
                        let field_src = {
                            let mut src = src.as_ptr() as usize;
                            src += old_offset;
                            src as *mut u8
                        };

                        unsafe {
                            std::ptr::copy_nonoverlapping(
                                field_src,
                                field_dest,
                                new_ty.layout.size(),
                            )
                        };
                    }
                    mapping::Action::Insert => {
                        if !new_ty.is_stack_allocated() {
                            let object = alloc_obj(new_ty.clone());

                            // We want to return a pointer to the `ObjectInfo`, to be used as
                            // handle.
                            let handle = (object.as_ref().deref() as *const _ as RawGcPtr).into();

                            // Zero-initialize heap-allocated object
                            unsafe {
                                std::ptr::write_bytes(object.data.ptr, 0, new_ty.layout.size())
                            };

                            // Write handle to field
                            let field_dest = field_dest.cast::<GcPtr>();
                            unsafe {
                                *field_dest = handle;
                            }

                            new_allocations.push(object);
                        } else {
                            // Use the previously zero-initialized value
                        }
                    }
                }
            }
        }
    }
}

/// Coloring used in the Mark Sweep phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Color {
    /// A white object has not been seen yet by the mark phase
    White,

    /// A gray object has been seen by the mark phase but has not yet been visited
    Gray,

    /// A black object has been visited by the mark phase
    Black,
}

/// An indirection table that stores the address to the actual memory, the type of the object and
/// meta information.
#[repr(C)]
struct ObjectInfo {
    pub data: ObjectInfoData,
    pub roots: u32,
    pub color: Color,
    pub ty: Arc<TypeInfo>,
}

#[repr(C)]
union ObjectInfoData {
    pub ptr: *mut u8,
    pub array: *mut ArrayHeader,
}

/// An `ObjectInfo` is thread-safe.
unsafe impl Send for ObjectInfo {}
unsafe impl Sync for ObjectInfo {}

impl From<GcPtr> for *const ObjectInfo {
    fn from(ptr: GcPtr) -> Self {
        ptr.as_ptr() as Self
    }
}

impl From<GcPtr> for *mut ObjectInfo {
    fn from(ptr: GcPtr) -> Self {
        ptr.as_ptr() as Self
    }
}

impl From<*const ObjectInfo> for GcPtr {
    fn from(info: *const ObjectInfo) -> Self {
        (info as RawGcPtr).into()
    }
}

impl From<*mut ObjectInfo> for GcPtr {
    fn from(info: *mut ObjectInfo) -> Self {
        (info as RawGcPtr).into()
    }
}

impl ObjectInfo {
    /// Returns the layout of the data pointed to by data
    pub fn layout(&self) -> Layout {
        match &self.ty.data {
            TypeInfoData::Struct(_) | TypeInfoData::Primitive(_) | TypeInfoData::Pointer(_) => {
                self.ty.layout
            }
            TypeInfoData::Array(array) => {
                let elem_count = unsafe { (*self.data.array).capacity };
                let elem_layout = repeat_layout(array.element_ty.layout, elem_count)
                    .expect("unable to determine layout of array elements");
                let (layout, _) = Layout::new::<ArrayHeader>()
                    .extend(elem_layout)
                    .expect("unable to determine layout of array");
                layout
            }
        }
    }
}

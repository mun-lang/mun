use memory::{
    gc::{self, HasIndirectionPtr},
    ArrayMemoryLayout, ArrayType, CompositeTypeKind, TypeMemory,
};
use std::{alloc::Layout, hash::Hash, ptr::NonNull};

/// `UnsafeTypeInfo` is a type that wraps a `NonNull<TypeInfo>` and indicates unsafe interior
/// operations on the wrapped `TypeInfo`. The unsafety originates from uncertainty about the
/// lifetime of the wrapped `TypeInfo`.
///
/// Rust lifetime rules do not allow separate lifetimes for struct fields, but we can make `unsafe`
/// guarantees about their lifetimes. Thus the `UnsafeTypeInfo` type is the only legal way to obtain
/// shared references to the wrapped `TypeInfo`.
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct UnsafeTypeInfo(NonNull<abi::TypeInfo>);

impl UnsafeTypeInfo {
    /// Constructs a new instance of `UnsafeTypeInfo`, which will wrap the specified `type_info`
    /// pointer.
    ///
    /// All access to the inner value through methods is `unsafe`.
    pub fn new(type_info: NonNull<abi::TypeInfo>) -> Self {
        Self(type_info)
    }

    /// Unwraps the value.
    pub fn into_inner(self) -> NonNull<abi::TypeInfo> {
        self.0
    }
}

impl PartialEq for UnsafeTypeInfo {
    fn eq(&self, other: &Self) -> bool {
        unsafe { *self.0.as_ref() == *other.0.as_ref() }
    }
}

impl Eq for UnsafeTypeInfo {}

impl Hash for UnsafeTypeInfo {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        unsafe { self.0.as_ref().hash(state) };
    }
}

impl memory::TypeDesc for UnsafeTypeInfo {
    fn name(&self) -> &str {
        unsafe { self.0.as_ref().name() }
    }

    fn guid(&self) -> &abi::Guid {
        unsafe { &self.0.as_ref().guid }
    }
}

impl memory::CompositeType for UnsafeTypeInfo {
    type ArrayType = WrappedAbiArrayInfo;
    type StructType = WrappedAbiStructInfo;

    fn group(&self) -> CompositeTypeKind<'_, Self::ArrayType, Self::StructType> {
        match unsafe { &self.0.as_ref().data } {
            TypeInfoData::Primitive => CompositeTypeKind::Primitive,
            TypeInfoData::Struct(s) => CompositeTypeKind::Struct(unsafe {
                std::mem::transmute::<&abi::StructInfo, &WrappedAbiStructInfo>(s)
            }),
            TypeInfoData::Array(a) => CompositeTypeKind::Array(unsafe {
                std::mem::transmute::<&abi::ArrayInfo, &WrappedAbiArrayInfo>(a)
            }),
        }
    }
}

/// This is a super hacky unsafe way to be able to implement traits from `mun_memory` for types
/// defined in `mun_abi`.
#[repr(transparent)]
pub struct WrappedAbiStructInfo(pub abi::StructInfo);

/// This is a super hacky unsafe way to be able to implement traits from `mun_memory` for types
/// defined in `mun_abi`.
#[repr(transparent)]
pub struct WrappedAbiArrayInfo(pub abi::ArrayInfo);

impl memory::StructFields<UnsafeTypeInfo> for WrappedAbiStructInfo {
    fn fields(&self) -> Vec<(&str, UnsafeTypeInfo)> {
        self.0
            .field_names()
            .zip(self.0.field_types().iter().map(|ty| {
                // Safety: `ty` is a shared reference, so is guaranteed to not be `ptr::null()`.
                UnsafeTypeInfo::new(unsafe {
                    NonNull::new_unchecked(*ty as *const abi::TypeInfo as *mut _)
                })
            }))
            .collect()
    }
}
impl memory::StructFieldLayout for WrappedAbiStructInfo {
    fn offsets(&self) -> &[u16] {
        self.0.field_offsets()
    }
}

impl memory::ArrayType<UnsafeTypeInfo> for WrappedAbiArrayInfo {
    fn element_type(&self) -> UnsafeTypeInfo {
        UnsafeTypeInfo::new(unsafe {
            NonNull::new_unchecked(self.0.element_type() as *const abi::TypeInfo as *mut _)
        })
    }
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

/// An iterator type to iterate over the elements of an array.
pub struct ArrayIterator {
    element_offset: NonNull<u8>,
    element_stride: usize,
    remaining: usize,
}

impl Iterator for ArrayIterator {
    type Item = NonNull<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining > 0 {
            let result = self.element_offset;
            self.element_offset = unsafe {
                NonNull::new_unchecked(self.element_offset.as_ptr().add(self.element_stride))
            };
            self.remaining -= 1;
            Some(result)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }

    fn last(self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        if self.remaining > 0 {
            Some(unsafe {
                NonNull::new_unchecked(self.element_offset.as_ptr().add(self.remaining - 1))
            })
        } else {
            None
        }
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        if self.remaining > n {
            let result = self.element_offset;
            self.element_offset = unsafe {
                NonNull::new_unchecked(self.element_offset.as_ptr().add(self.element_stride * n))
            };
            self.remaining -= n;
            Some(result)
        } else {
            self.remaining = 0;
            None
        }
    }
}

impl ExactSizeIterator for ArrayIterator {}

impl WrappedAbiArrayInfo {
    /// Returns the layout of a single element in the array
    fn element_layout(&self) -> Layout {
        let element_ty = self.element_type();
        let raw_element_ty = unsafe { element_ty.0.as_ref() };
        match raw_element_ty.data {
            TypeInfoData::Primitive | TypeInfoData::Struct(_)
                if element_ty.is_stack_allocated() =>
            {
                element_ty.layout()
            }
            _ => Layout::new::<GcPtr>(),
        }
    }

    /// Returns the layout of the size and the offset from the start of the array block in bytes
    const fn size_layout() -> (Layout, usize) {
        (Layout::new::<usize>(), 0)
    }

    /// Returns the offset of the first element of the array from the start of the array block in
    /// bytes.
    fn element_offset(&self) -> usize {
        let (size_layout, offset) = Self::size_layout();
        let element_layout = self.element_layout();

        let element_align = element_layout.align();
        let size_len = size_layout.size();

        let unaligned_element_offset = offset + size_len;
        let len_rounded_up = unaligned_element_offset
            .wrapping_add(element_align)
            .wrapping_sub(1)
            & !element_align.wrapping_sub(1);
        len_rounded_up.wrapping_sub(unaligned_element_offset)
    }

    /// Returns the number of bytes between each element
    fn element_stride(&self) -> usize {
        let element_layout = self.element_layout();
        let size = element_layout.size();
        let align = element_layout.align();
        let len_rounded_up = size.wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
        len_rounded_up.wrapping_sub(size)
    }
}

impl memory::ArrayMemoryLayout for WrappedAbiArrayInfo {
    type ElementIterator = ArrayIterator;

    fn layout(&self, n: usize) -> Layout {
        let elements_layout = repeat_layout(self.element_layout(), n)
            .expect("unable to create a memory layout for array");
        let length_layout = Layout::new::<usize>();

        length_layout
            .extend(elements_layout)
            .expect("unable to construct memory layout for array value")
            .0
    }

    unsafe fn data_layout(&self, ptr: NonNull<u8>) -> Layout {
        let len = self.retrieve_length(ptr);
        self.layout(len)
    }

    unsafe fn retrieve_length(&self, ptr: NonNull<u8>) -> usize {
        *ptr.cast().as_ptr()
    }

    unsafe fn elements(&self, ptr: NonNull<u8>) -> Self::ElementIterator {
        ArrayIterator {
            element_offset: NonNull::new_unchecked(ptr.as_ptr().add(self.element_offset())),
            element_stride: self.element_stride(),
            remaining: self.retrieve_length(ptr),
        }
    }

    unsafe fn store_length(&self, ptr: NonNull<u8>, n: usize) {
        *ptr.cast().as_ptr() = n;
    }
}

impl memory::HasDynamicMemoryLayout for UnsafeTypeInfo {
    unsafe fn layout(&self, ptr: NonNull<u8>) -> Layout {
        match &self.0.as_ref().data {
            TypeInfoData::Primitive | TypeInfoData::Struct(_) => TypeMemory::layout(self),
            TypeInfoData::Array(a) => {
                std::mem::transmute::<&abi::ArrayInfo, &WrappedAbiArrayInfo>(a).data_layout(ptr)
            }
        }
    }
}

unsafe impl Send for UnsafeTypeInfo {}
unsafe impl Sync for UnsafeTypeInfo {}

impl memory::TypeMemory for UnsafeTypeInfo {
    fn layout(&self) -> Layout {
        let ty = unsafe { self.0.as_ref() };
        Layout::from_size_align(ty.size_in_bytes(), ty.alignment())
            .unwrap_or_else(|_| panic!("invalid layout from Mun Type: {:?}", ty))
    }

    fn is_stack_allocated(&self) -> bool {
        unsafe {
            self.0
                .as_ref()
                .as_struct()
                .map_or(true, |s| s.memory_kind == abi::StructMemoryKind::Value)
        }
    }
}

fn trace<F: FnMut(GcPtr)>(ty: &abi::TypeInfo, value_ptr: NonNull<u8>, f: &mut F, is_root: bool) {
    match &ty.data {
        TypeInfoData::Primitive => {
            // Nothing to do
        }
        TypeInfoData::Struct(struct_info) => {
            if struct_info.memory_kind == abi::StructMemoryKind::Gc && !is_root {
                f(unsafe { *value_ptr.cast::<GcPtr>().as_ptr() })
            } else {
                for (&field_type, &field_offset) in struct_info
                    .field_types()
                    .iter()
                    .zip(struct_info.field_offsets().iter())
                {
                    let field_ptr = unsafe {
                        NonNull::new_unchecked(value_ptr.as_ptr().add(field_offset as usize))
                    };
                    trace(field_type, field_ptr, f, false)
                }
            }
        }
        TypeInfoData::Array(array_info) => {
            let array_ty =
                unsafe { std::mem::transmute::<&abi::ArrayInfo, &WrappedAbiArrayInfo>(array_info) };
            let element_ty = unsafe { array_ty.element_type().0.as_ref() };
            for element_ptr in unsafe { array_ty.elements(value_ptr) } {
                trace(element_ty, element_ptr, f, false)
            }
        }
    }
}

impl gc::TypeTrace for UnsafeTypeInfo {
    fn trace<F: FnMut(GcPtr)>(&self, obj: GcPtr, mut f: F) {
        trace(
            unsafe { self.0.as_ref() },
            unsafe { NonNull::new_unchecked(obj.deref::<u8>() as *mut u8) },
            &mut f,
            true,
        )
    }
}

/// Defines the garbage collector used by the `Runtime`.
pub type GarbageCollector = gc::MarkSweep<UnsafeTypeInfo, gc::NoopObserver<gc::Event>>;

use abi::TypeInfoData;
pub use gc::GcPtr;
use std::alloc::LayoutError;

pub type GcRootPtr = gc::GcRootPtr<UnsafeTypeInfo, GarbageCollector>;

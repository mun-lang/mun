use memory::{
    gc::{self, HasIndirectionPtr},
    TypeGroup,
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

    fn group(&self) -> TypeGroup {
        TypeGroup::from(unsafe { &self.0.as_ref().data })
    }
}

impl memory::TypeFields<UnsafeTypeInfo> for UnsafeTypeInfo {
    fn fields(&self) -> Vec<(&str, Self)> {
        if let Some(s) = unsafe { self.0.as_ref().as_struct() } {
            s.field_names()
                .zip(s.field_types().iter().map(|ty| {
                    // Safety: `ty` is a shared reference, so is guaranteed to not be `ptr::null()`.
                    UnsafeTypeInfo::new(unsafe {
                        NonNull::new_unchecked(*ty as *const abi::TypeInfo as *mut _)
                    })
                }))
                .collect()
        } else {
            Vec::new()
        }
    }

    fn offsets(&self) -> &[u16] {
        if let Some(s) = unsafe { self.0.as_ref().as_struct() } {
            s.field_offsets()
        } else {
            &[]
        }
    }
}

unsafe impl Send for UnsafeTypeInfo {}
unsafe impl Sync for UnsafeTypeInfo {}

pub struct Trace {
    obj: GcPtr,
    ty: UnsafeTypeInfo,
    index: usize,
}

impl Iterator for Trace {
    type Item = GcPtr;

    fn next(&mut self) -> Option<Self::Item> {
        let struct_ty = unsafe { self.ty.0.as_ref() }.as_struct()?;
        let field_count = struct_ty.field_types().len();
        while self.index < field_count {
            let index = self.index;
            self.index += 1;

            let field_ty = struct_ty.field_types()[index];
            if let Some(field_struct_ty) = field_ty.as_struct() {
                if field_struct_ty.memory_kind == abi::StructMemoryKind::GC {
                    let offset = struct_ty.field_offsets()[index];
                    return Some(unsafe {
                        *self.obj.deref::<u8>().add(offset as usize).cast::<GcPtr>()
                    });
                }
            }
        }
        None
    }
}

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

impl gc::TypeTrace for UnsafeTypeInfo {
    type Trace = Trace;

    fn trace(&self, obj: GcPtr) -> Self::Trace {
        Trace {
            ty: *self,
            obj,
            index: 0,
        }
    }
}

/// Defines the garbage collector used by the `Runtime`.
pub type GarbageCollector = gc::MarkSweep<UnsafeTypeInfo, gc::NoopObserver<gc::Event>>;

pub use gc::GcPtr;
pub type GcRootPtr = gc::GcRootPtr<UnsafeTypeInfo, GarbageCollector>;

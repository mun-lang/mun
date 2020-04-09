use gc::HasIndirectionPtr;
use std::alloc::Layout;

#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct RawTypeInfo(*const abi::TypeInfo);

impl RawTypeInfo {
    /// Returns the inner `TypeInfo` pointer.
    ///
    /// # Safety
    ///
    /// This method is unsafe because there are no guarantees about the lifetime of the inner
    /// pointer.
    pub unsafe fn inner(&self) -> *const abi::TypeInfo {
        self.0
    }
}

impl Into<RawTypeInfo> for *const abi::TypeInfo {
    fn into(self) -> RawTypeInfo {
        RawTypeInfo(self)
    }
}

unsafe impl Send for RawTypeInfo {}
unsafe impl Sync for RawTypeInfo {}

pub struct Trace {
    obj: GcPtr,
    ty: RawTypeInfo,
    index: usize,
}

impl Iterator for Trace {
    type Item = GcPtr;

    fn next(&mut self) -> Option<Self::Item> {
        let struct_ty = unsafe { self.ty.0.as_ref() }.unwrap().as_struct()?;
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

impl gc::Type for RawTypeInfo {
    type Trace = Trace;

    fn layout(&self) -> Layout {
        let ty = unsafe { &*self.0 };
        Layout::from_size_align(ty.size_in_bytes(), ty.alignment())
            .expect("invalid layout from Mun Type")
    }

    fn trace(&self, obj: GcPtr) -> Self::Trace {
        Trace {
            ty: self.clone(),
            obj,
            index: 0,
        }
    }
}

/// Defines the garbage collector used by the `Runtime`.
pub type GarbageCollector = gc::MarkSweep<RawTypeInfo, gc::NoopObserver<gc::Event>>;

pub use gc::GcPtr;
pub type GcRootPtr = gc::GcRootPtr<RawTypeInfo, GarbageCollector>;

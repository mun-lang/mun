use gc::HasIndirectionPtr;
use std::{alloc::Layout, hash::Hash};

#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct RawTypeInfo(*const abi::TypeInfo);

impl RawTypeInfo {
    /// Returns the inner `TypeInfo` pointer.
    ///
    /// # Safety
    ///
    /// This method is unsafe because there are no guarantees about the lifetime of the inner
    /// pointer.
    pub unsafe fn inner(self) -> *const abi::TypeInfo {
        self.0
    }
}

impl PartialEq for RawTypeInfo {
    fn eq(&self, other: &Self) -> bool {
        let this = unsafe { &*self.0 };
        let other = unsafe { &*other.0 };
        *this == *other
    }
}

impl Eq for RawTypeInfo {}

impl Hash for RawTypeInfo {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let this = unsafe { &*self.0 };
        this.hash(state);
    }
}

impl memory::TypeDesc for RawTypeInfo {
    fn name(&self) -> &str {
        let this = unsafe { &*self.0 };
        this.name()
    }
    fn guid(&self) -> &abi::Guid {
        let this = unsafe { &*self.0 };
        &this.guid
    }
    fn group(&self) -> abi::TypeGroup {
        let this = unsafe { &*self.0 };
        this.group
    }
}

impl memory::TypeFields<RawTypeInfo> for RawTypeInfo {
    fn fields(&self) -> Vec<(&str, Self)> {
        let this = unsafe { &*self.0 };
        if let Some(s) = this.as_struct() {
            s.field_names()
                .zip(
                    s.field_types()
                        .iter()
                        .map(|ty| (*ty as *const abi::TypeInfo).into()),
                )
                .collect()
        } else {
            Vec::new()
        }
    }

    fn offsets(&self) -> &[u16] {
        let this = unsafe { &*self.0 };
        if let Some(s) = this.as_struct() {
            s.field_offsets()
        } else {
            &[]
        }
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

impl memory::TypeLayout for RawTypeInfo {
    fn layout(&self) -> Layout {
        let ty = unsafe { &*self.0 };
        Layout::from_size_align(ty.size_in_bytes(), ty.alignment())
            .unwrap_or_else(|_| panic!("invalid layout from Mun Type: {:?}", ty))
    }
}

impl gc::TypeTrace for RawTypeInfo {
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
pub type GarbageCollector = gc::MarkSweep<RawTypeInfo, gc::NoopObserver<gc::Event>>;

pub use gc::GcPtr;
pub type GcRootPtr = gc::GcRootPtr<RawTypeInfo, GarbageCollector>;

use gc::HasGCHandlePtr;

#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct RawTypeInfo(*const abi::TypeInfo);

impl Into<RawTypeInfo> for *const abi::TypeInfo {
    fn into(self) -> RawTypeInfo {
        RawTypeInfo(self)
    }
}

unsafe impl Send for RawTypeInfo {}
unsafe impl Sync for RawTypeInfo {}

pub struct Trace {
    obj: GCHandle,
    ty: RawTypeInfo,
    index: usize,
}

impl Iterator for Trace {
    type Item = GCHandle;

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
                        *self
                            .obj
                            .get_ptr::<u8>()
                            .as_ptr()
                            .add(offset as usize)
                            .cast::<GCHandle>()
                    });
                }
            }
        }
        None
    }
}

impl gc::Type for RawTypeInfo {
    type Trace = Trace;

    fn size(&self) -> usize {
        unsafe { (*self.0).size_in_bytes() }
    }

    fn alignment(&self) -> usize {
        unsafe { (*self.0).alignment() }
    }

    fn trace(&self, obj: GCHandle) -> Self::Trace {
        Trace {
            ty: self.clone(),
            obj,
            index: 0,
        }
    }
}

/// Defines an allocator used by the `Runtime`
pub type Allocator = gc::MarkSweep<RawTypeInfo, gc::NoopObserver>;

pub use gc::GCHandle;

pub type GCRootHandle = gc::GCRootHandle<RawTypeInfo, Allocator>;

use crate::garbage_collector::{GCPtr, GCRootHandle};
use crate::{
    marshal::Marshal,
    reflection::{
        equals_argument_type, equals_return_type, ArgumentReflection, ReturnTypeReflection,
    },
    Runtime,
};
use gc::{GCRuntime, HasIndirectionPtr};
use std::cell::RefCell;
use std::ptr::{self, NonNull};
use std::rc::Rc;

/// Represents a Mun struct pointer.
///
/// A byte pointer is used to make pointer arithmetic easier.
#[repr(transparent)]
#[derive(Clone)]
pub struct RawStruct(GCPtr);

impl RawStruct {
    /// Returns a pointer to the struct memory.
    pub unsafe fn get_ptr(&self) -> *const u8 {
        self.0.deref()
    }
}

/// Type-agnostic wrapper for interoperability with a Mun struct.
/// TODO: Handle destruction of `struct(value)`
pub struct StructRef {
    runtime: Rc<RefCell<Runtime>>,
    handle: GCRootHandle,
    type_info: *const abi::TypeInfo,
    info: abi::StructInfo,
}

impl StructRef {
    /// Creates a struct that wraps a raw Mun struct.
    ///
    /// The provided [`TypeInfo`] must be for a struct type.
    fn new(runtime: Rc<RefCell<Runtime>>, type_info: &abi::TypeInfo, raw: RawStruct) -> StructRef {
        assert!(type_info.group.is_struct());

        let handle = {
            let runtime_ref = runtime.borrow();
            unsafe { GCRootHandle::new(runtime_ref.gc(), raw.0) }
        };
        Self {
            runtime,
            handle,
            type_info: type_info as *const abi::TypeInfo,
            info: type_info.as_struct().unwrap().clone(),
        }
    }

    /// Consumes the `Struct`, returning a raw Mun struct.
    pub fn into_raw(self) -> RawStruct {
        RawStruct(self.handle.handle())
    }

    /// Retrieves its struct information.
    pub fn info(&self) -> &abi::StructInfo {
        &self.info
    }

    /// Returns the type information of the struct
    pub fn type_info(&self) -> *const abi::TypeInfo {
        self.type_info
    }

    ///
    ///
    /// # Safety
    ///
    ///
    unsafe fn offset_unchecked<T>(&self, field_idx: usize) -> NonNull<T> {
        let offset = *self.info.field_offsets().get_unchecked(field_idx);
        // self.raw is never null
        NonNull::new_unchecked(
            self.handle
                .deref_mut::<u8>()
                .add(offset as usize)
                .cast::<T>(),
        )
    }

    /// Retrieves the value of the field corresponding to the specified `field_name`.
    pub fn get<T: ReturnTypeReflection>(&self, field_name: &str) -> Result<T, String> {
        let field_idx = abi::StructInfo::find_field_index(&self.info, field_name)?;
        let field_type = unsafe { self.info.field_types().get_unchecked(field_idx) };
        equals_return_type::<T>(&field_type).map_err(|(expected, found)| {
            format!(
                "Mismatched types for `{}::{}`. Expected: `{}`. Found: `{}`.",
                self.info.name(),
                field_name,
                expected,
                found,
            )
        })?;

        // If we found the `field_idx`, we are guaranteed to also have the `field_offset`
        let field_ptr = unsafe { self.offset_unchecked::<T::Marshalled>(field_idx) };
        Ok(Marshal::marshal_from_ptr(
            field_ptr,
            self.runtime.clone(),
            Some(*field_type),
        ))
    }

    /// Replaces the value of the field corresponding to the specified `field_name` and returns the
    /// old value.
    pub fn replace<T: ArgumentReflection>(
        &mut self,
        field_name: &str,
        value: T,
    ) -> Result<T, String> {
        let field_idx = abi::StructInfo::find_field_index(&self.info, field_name)?;
        let field_type = unsafe { self.info.field_types().get_unchecked(field_idx) };
        equals_argument_type(&field_type, &value).map_err(|(expected, found)| {
            format!(
                "Mismatched types for `{}::{}`. Expected: `{}`. Found: `{}`.",
                self.info.name(),
                field_name,
                expected,
                found,
            )
        })?;

        let field_ptr = unsafe { self.offset_unchecked::<T::Marshalled>(field_idx) };
        let old = Marshal::marshal_from_ptr(field_ptr, self.runtime.clone(), Some(*field_type));
        Marshal::marshal_to_ptr(value.marshal(), field_ptr, Some(*field_type));
        Ok(old)
    }

    /// Sets the value of the field corresponding to the specified `field_name`.
    pub fn set<T: ArgumentReflection>(&mut self, field_name: &str, value: T) -> Result<(), String> {
        let field_idx = abi::StructInfo::find_field_index(&self.info, field_name)?;
        let field_type = unsafe { self.info.field_types().get_unchecked(field_idx) };
        equals_argument_type(&field_type, &value).map_err(|(expected, found)| {
            format!(
                "Mismatched types for `{}::{}`. Expected: `{}`. Found: `{}`.",
                self.info.name(),
                field_name,
                expected,
                found,
            )
        })?;

        let field_ptr = unsafe { self.offset_unchecked::<T::Marshalled>(field_idx) };
        Marshal::marshal_to_ptr(value.marshal(), field_ptr, Some(*field_type));
        Ok(())
    }
}

impl ArgumentReflection for StructRef {
    type Marshalled = RawStruct;

    fn type_name(&self) -> &str {
        self.info.name()
    }

    fn marshal(self) -> Self::Marshalled {
        self.into_raw()
    }
}

impl ReturnTypeReflection for StructRef {
    type Marshalled = RawStruct;

    fn type_name() -> &'static str {
        "struct"
    }
}

impl Marshal<StructRef> for RawStruct {
    fn marshal_value(
        self,
        runtime: Rc<RefCell<Runtime>>,
        type_info: Option<&abi::TypeInfo>,
    ) -> StructRef {
        // `type_info` is only `None` for the `()` type
        StructRef::new(runtime, type_info.unwrap(), self)
    }

    fn marshal_from_ptr(
        ptr: NonNull<Self>,
        runtime: Rc<RefCell<Runtime>>,
        type_info: Option<&abi::TypeInfo>,
    ) -> StructRef {
        // `type_info` is only `None` for the `()` type
        let type_info = type_info.unwrap();
        let struct_info = type_info.as_struct().unwrap();

        // HACK: This is very hacky since we know nothing about the lifetime of abi::TypeInfo.
        let type_info_ptr = (type_info as *const abi::TypeInfo).into();

        // Copy the contents of the struct based on what kind of pointer we are dealing with
        let gc_handle = if struct_info.memory_kind == abi::StructMemoryKind::Value {
            // If this case the passed in `ptr` is a pointer to a value struct so `ptr` points to a
            // struct value.

            // Create a new object using the runtime's intrinsic
            let gc_handle = {
                let runtime_ref = runtime.borrow();
                runtime_ref.gc().alloc(type_info_ptr)
            };

            // Construct
            let src = ptr.cast::<u8>().as_ptr() as *const _;
            let dest = unsafe { gc_handle.deref_mut::<u8>() };
            let size = type_info.size_in_bytes();
            unsafe { ptr::copy_nonoverlapping(src, dest, size as usize) };

            gc_handle
        } else {
            // If this case the passed in `ptr` is a pointer to a gc struct so `ptr` points to a
            // GCPtr.

            unsafe { *ptr.cast::<GCPtr>().as_ptr() }
        };

        StructRef::new(runtime, type_info, RawStruct(gc_handle))
    }

    fn marshal_to_ptr(value: RawStruct, mut ptr: NonNull<Self>, type_info: Option<&abi::TypeInfo>) {
        // `type_info` is only `None` for the `()` type
        let type_info = type_info.unwrap();

        let struct_info = type_info.as_struct().unwrap();
        if struct_info.memory_kind == abi::StructMemoryKind::Value {
            let dest = ptr.cast::<u8>().as_ptr();
            let size = type_info.size_in_bytes();
            unsafe { ptr::copy_nonoverlapping(value.get_ptr(), dest, size as usize) };
        } else {
            unsafe { *ptr.as_mut() = value };
        }
    }
}

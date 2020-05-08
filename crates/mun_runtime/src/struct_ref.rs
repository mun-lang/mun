use crate::garbage_collector::{GcPtr, GcRootPtr, UnsafeTypeInfo};
use crate::{
    marshal::Marshal,
    reflection::{
        equals_argument_type, equals_return_type, ArgumentReflection, ReturnTypeReflection,
    },
    Runtime,
};
use memory::gc::{GcRuntime, HasIndirectionPtr};
use std::cell::RefCell;
use std::{
    ptr::{self, NonNull},
    rc::Rc,
};

/// Represents a Mun struct pointer.
#[repr(transparent)]
#[derive(Clone)]
pub struct RawStruct(GcPtr);

impl RawStruct {
    /// Returns a pointer to the struct memory.
    pub unsafe fn get_ptr(&self) -> *const u8 {
        self.0.deref()
    }
}

/// Type-agnostic wrapper for interoperability with a Mun struct.
#[derive(Clone)]
pub struct StructRef {
    handle: GcRootPtr,
    runtime: Rc<RefCell<Runtime>>,
}

impl StructRef {
    /// Creates a `StructRef` that wraps a raw Mun struct.
    fn new(runtime: Rc<RefCell<Runtime>>, raw: RawStruct) -> Self {
        let handle = {
            let runtime_ref = runtime.borrow();
            // Safety: The type returned from `ptr_type` is guaranteed to live at least as long as
            // `Runtime` does not change. As we hold a shared reference to `Runtime`, this is safe.
            assert!(unsafe {
                runtime_ref
                    .gc()
                    .ptr_type(raw.0)
                    .into_inner()
                    .as_ref()
                    .group
                    .is_struct()
            });

            GcRootPtr::new(&runtime_ref.gc, raw.0)
        };

        Self { runtime, handle }
    }

    /// Consumes the `StructRef`, returning a raw Mun struct.
    pub fn into_raw(self) -> RawStruct {
        RawStruct(self.handle.handle())
    }

    /// Returns the type information of the struct.
    pub fn type_info<'r>(struct_ref: &Self, runtime_ref: &'r Runtime) -> &'r abi::TypeInfo {
        // Safety: The type returned from `ptr_type` is guaranteed to live at least as long as
        // `Runtime` does not change. As the lifetime of `TypeInfo` is tied to the lifetime of
        // `Runtime`, this is safe.
        unsafe {
            &*runtime_ref
                .gc
                .ptr_type(struct_ref.handle.handle())
                .into_inner()
                .as_ptr()
        }
    }

    ///
    ///
    /// # Safety
    ///
    ///
    unsafe fn field_offset_unchecked<T>(
        &self,
        struct_info: &abi::StructInfo,
        field_idx: usize,
    ) -> NonNull<T> {
        let offset = *struct_info.field_offsets().get_unchecked(field_idx);
        // self.raw is never null
        NonNull::new_unchecked(self.handle.deref::<u8>().add(offset as usize).cast::<T>() as *mut _)
    }

    /// Retrieves the value of the field corresponding to the specified `field_name`.
    pub fn get<T: ReturnTypeReflection>(&self, field_name: &str) -> Result<T, String> {
        let runtime_ref = self.runtime.borrow();
        let type_info = Self::type_info(self, &runtime_ref);

        // Safety: `as_struct` is guaranteed to return `Some` for `StructRef`s.
        let struct_info = type_info.as_struct().unwrap();
        let field_idx =
            abi::StructInfo::find_field_index(type_info.name(), struct_info, field_name)?;

        // Safety: If we found the `field_idx`, we are guaranteed to also have the `field_type` and
        // `field_offset`.
        let field_type = unsafe { struct_info.field_types().get_unchecked(field_idx) };
        equals_return_type::<T>(field_type).map_err(|(expected, found)| {
            format!(
                "Mismatched types for `{}::{}`. Expected: `{}`. Found: `{}`.",
                type_info.name(),
                field_name,
                expected,
                found,
            )
        })?;

        // If we found the `field_idx`, we are guaranteed to also have the `field_offset`
        let field_ptr =
            unsafe { self.field_offset_unchecked::<T::Marshalled>(struct_info, field_idx) };
        Ok(Marshal::marshal_from_ptr(
            field_ptr,
            self.runtime.clone(),
            Some(field_type),
        ))
    }

    /// Replaces the value of the field corresponding to the specified `field_name` and returns the
    /// old value.
    pub fn replace<T: ArgumentReflection>(
        &mut self,
        field_name: &str,
        value: T,
    ) -> Result<T, String> {
        let runtime_ref = self.runtime.borrow();
        let type_info = Self::type_info(self, &runtime_ref);

        // Safety: `as_struct` is guaranteed to return `Some` for `StructRef`s.
        let struct_info = type_info.as_struct().unwrap();
        let field_idx =
            abi::StructInfo::find_field_index(type_info.name(), struct_info, field_name)?;

        // Safety: If we found the `field_idx`, we are guaranteed to also have the `field_type` and
        // `field_offset`.
        let field_type = unsafe { struct_info.field_types().get_unchecked(field_idx) };
        equals_argument_type(&runtime_ref, field_type, &value).map_err(|(expected, found)| {
            format!(
                "Mismatched types for `{}::{}`. Expected: `{}`. Found: `{}`.",
                type_info.name(),
                field_name,
                expected,
                found,
            )
        })?;

        let field_ptr =
            unsafe { self.field_offset_unchecked::<T::Marshalled>(struct_info, field_idx) };
        let old = Marshal::marshal_from_ptr(field_ptr, self.runtime.clone(), Some(field_type));
        Marshal::marshal_to_ptr(value.marshal(), field_ptr, Some(field_type));
        Ok(old)
    }

    /// Sets the value of the field corresponding to the specified `field_name`.
    pub fn set<T: ArgumentReflection>(&mut self, field_name: &str, value: T) -> Result<(), String> {
        let runtime_ref = self.runtime.borrow();
        let type_info = Self::type_info(self, &runtime_ref);

        // Safety: `as_struct` is guaranteed to return `Some` for `StructRef`s.
        let struct_info = type_info.as_struct().unwrap();
        let field_idx =
            abi::StructInfo::find_field_index(type_info.name(), struct_info, field_name)?;

        // Safety: If we found the `field_idx`, we are guaranteed to also have the `field_type` and
        // `field_offset`.
        let field_type = unsafe { struct_info.field_types().get_unchecked(field_idx) };
        equals_argument_type(&runtime_ref, field_type, &value).map_err(|(expected, found)| {
            format!(
                "Mismatched types for `{}::{}`. Expected: `{}`. Found: `{}`.",
                type_info.name(),
                field_name,
                expected,
                found,
            )
        })?;

        let field_ptr =
            unsafe { self.field_offset_unchecked::<T::Marshalled>(struct_info, field_idx) };
        Marshal::marshal_to_ptr(value.marshal(), field_ptr, Some(field_type));
        Ok(())
    }
}

impl ArgumentReflection for StructRef {
    type Marshalled = RawStruct;

    fn type_guid(&self, runtime: &Runtime) -> abi::Guid {
        // Safety: The type returned from `ptr_type` is guaranteed to live at least as long as
        // `Runtime` does not change. As we hold a shared reference to `Runtime`, this is safe.
        unsafe {
            runtime
                .gc()
                .ptr_type(self.handle.handle())
                .into_inner()
                .as_ref()
                .guid
        }
    }

    fn type_name(&self, runtime: &Runtime) -> &str {
        // Safety: The type returned from `ptr_type` is guaranteed to live at least as long as
        // `Runtime` does not change. As we hold a shared reference to `Runtime`, this is safe.
        unsafe {
            (&*runtime
                .gc()
                .ptr_type(self.handle.handle())
                .into_inner()
                .as_ptr())
                .name()
        }
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
    fn marshal_value(self, runtime: Rc<RefCell<Runtime>>) -> StructRef {
        StructRef::new(runtime, self)
    }

    fn marshal_from_ptr(
        ptr: NonNull<Self>,
        runtime: Rc<RefCell<Runtime>>,
        type_info: Option<&abi::TypeInfo>,
    ) -> StructRef {
        // `type_info` is only `None` for the `()` type
        let type_info = type_info.unwrap();
        let struct_info = type_info.as_struct().unwrap();

        // Copy the contents of the struct based on what kind of pointer we are dealing with
        let gc_handle = if struct_info.memory_kind == abi::StructMemoryKind::Value {
            // For a value struct, `ptr` points to a struct value.

            // Create a new object using the runtime's intrinsic
            let mut gc_handle = {
                let runtime_ref = runtime.borrow();
                runtime_ref.gc().alloc(
                    // Safety: `ty` is a shared reference, so is guaranteed to not be `ptr::null()`.
                    UnsafeTypeInfo::new(unsafe {
                        NonNull::new_unchecked(type_info as *const abi::TypeInfo as *mut _)
                    }),
                )
            };

            // Construct
            let src = ptr.cast::<u8>().as_ptr() as *const _;
            let dest = unsafe { gc_handle.deref_mut::<u8>() };
            let size = type_info.size_in_bytes();
            unsafe { ptr::copy_nonoverlapping(src, dest, size) };

            gc_handle
        } else {
            // For a gc struct, `ptr` points to a `GcPtr`.
            unsafe { *ptr.cast::<GcPtr>().as_ptr() }
        };

        StructRef::new(runtime, RawStruct(gc_handle))
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

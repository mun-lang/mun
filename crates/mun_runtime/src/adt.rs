use crate::{
    garbage_collector::GcRootPtr,
    marshal::Marshal,
    reflection::{ArgumentReflection, ReturnTypeReflection},
    GarbageCollector, Runtime,
};
use memory::{
    gc::{GcPtr, GcRuntime, HasIndirectionPtr},
    Type,
};
use std::{
    ptr::{self, NonNull},
    sync::Arc,
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

/// Type-agnostic wrapper for interoperability with a Mun struct. This is merely a reference to the
/// Mun struct, that will be garbage collected unless it is rooted.
#[derive(Clone)]
pub struct StructRef<'s> {
    raw: RawStruct,
    runtime: &'s Runtime,
}

impl<'s> StructRef<'s> {
    /// Creates a `StructRef` that wraps a raw Mun struct.
    fn new<'r>(raw: RawStruct, runtime: &'r Runtime) -> Self
    where
        'r: 's,
    {
        Self { raw, runtime }
    }

    /// Consumes the `StructRef`, returning a raw Mun struct.
    pub fn into_raw(self) -> RawStruct {
        self.raw
    }

    /// Roots the `StructRef`.
    pub fn root(self) -> RootedStruct {
        RootedStruct::new(&self.runtime.gc, self.raw)
    }

    /// Returns the type information of the struct.
    pub fn type_info(&self) -> Arc<Type> {
        self.runtime.gc.ptr_type(self.raw.0)
    }

    /// Returns the struct's field at the specified `offset`.
    ///
    /// # Safety
    ///
    /// The offset must be the location of a variable of type T.
    unsafe fn get_field_ptr_unchecked<T>(&self, offset: usize) -> NonNull<T> {
        // SAFETY: self.raw's memory pointer is never null
        let ptr = self.raw.get_ptr();

        NonNull::new_unchecked(ptr.add(offset).cast::<T>() as *mut T)
    }

    /// Retrieves the value of the field corresponding to the specified `field_name`.
    pub fn get<T: ReturnTypeReflection + Marshal<'s>>(&self, field_name: &str) -> Result<T, String>
    where
        T: 's,
    {
        let type_info = self.type_info();

        // Safety: `as_struct` is guaranteed to return `Some` for `StructRef`s.
        let struct_info = type_info.as_struct().unwrap();

        let field_info = struct_info.find_field_by_name(field_name).ok_or_else(|| {
            format!(
                "Struct `{}` does not contain field `{}`.",
                type_info.name(),
                field_name
            )
        })?;

        if !T::accepts_type(&field_info.type_info) {
            return Err(format!(
                "Mismatched types for `{}::{}`. Expected: `{}`. Found: `{}`.",
                type_info.name(),
                field_name,
                T::type_hint(),
                field_info.type_info.name(),
            ));
        };

        // SAFETY: The offset in the ABI is always valid.
        let field_ptr =
            unsafe { self.get_field_ptr_unchecked::<T::MunType>(usize::from(field_info.offset)) };
        Ok(Marshal::marshal_from_ptr(
            field_ptr,
            self.runtime,
            &field_info.type_info,
        ))
    }

    /// Replaces the value of the field corresponding to the specified `field_name` and returns the
    /// old value.
    pub fn replace<T: ArgumentReflection + Marshal<'s>>(
        &mut self,
        field_name: &str,
        value: T,
    ) -> Result<T, String>
    where
        T: 's,
    {
        let type_info = self.type_info();

        // Safety: `as_struct` is guaranteed to return `Some` for `StructRef`s.
        let struct_info = type_info.as_struct().unwrap();

        let field_info = struct_info.find_field_by_name(field_name).ok_or_else(|| {
            format!(
                "Struct `{}` does not contain field `{}`.",
                type_info.name(),
                field_name
            )
        })?;

        let value_type = value.type_info(self.runtime);
        if field_info.type_info != value_type {
            return Err(format!(
                "Mismatched types for `{}::{}`. Expected: `{}`. Found: `{}`.",
                type_info.name(),
                field_name,
                value_type.name(),
                field_info.type_info
            ));
        }

        // SAFETY: The offset in the ABI is always valid.
        let field_ptr =
            unsafe { self.get_field_ptr_unchecked::<T::MunType>(usize::from(field_info.offset)) };
        let old = Marshal::marshal_from_ptr(field_ptr, self.runtime, &field_info.type_info);
        Marshal::marshal_to_ptr(value, field_ptr, &field_info.type_info);
        Ok(old)
    }

    /// Sets the value of the field corresponding to the specified `field_name`.
    pub fn set<T: ArgumentReflection + Marshal<'s>>(
        &mut self,
        field_name: &str,
        value: T,
    ) -> Result<(), String> {
        let type_info = self.type_info();

        // Safety: `as_struct` is guaranteed to return `Some` for `StructRef`s.
        let struct_info = type_info.as_struct().unwrap();

        let field_info = struct_info.find_field_by_name(field_name).ok_or_else(|| {
            format!(
                "Struct `{}` does not contain field `{}`.",
                type_info.name(),
                field_name
            )
        })?;

        let value_type = value.type_info(self.runtime);
        if field_info.type_info != value_type {
            return Err(format!(
                "Mismatched types for `{}::{}`. Expected: `{}`. Found: `{}`.",
                type_info.name(),
                field_name,
                value_type.name(),
                field_info.type_info
            ));
        }

        // SAFETY: The offset in the ABI is always valid.
        let field_ptr =
            unsafe { self.get_field_ptr_unchecked::<T::MunType>(usize::from(field_info.offset)) };
        Marshal::marshal_to_ptr(value, field_ptr, &field_info.type_info);
        Ok(())
    }
}

impl<'r> ArgumentReflection for StructRef<'r> {
    fn type_info(&self, _runtime: &Runtime) -> Arc<Type> {
        self.type_info()
    }
}

impl<'s> Marshal<'s> for StructRef<'s> {
    type MunType = RawStruct;

    fn marshal_from<'r>(value: Self::MunType, runtime: &'r Runtime) -> Self
    where
        'r: 's,
    {
        StructRef::new(value, runtime)
    }

    fn marshal_into<'r>(self) -> Self::MunType {
        self.into_raw()
    }

    fn marshal_from_ptr<'r>(
        ptr: NonNull<Self::MunType>,
        runtime: &'r Runtime,
        type_info: &Arc<Type>,
    ) -> StructRef<'s>
    where
        Self: 's,
        'r: 's,
    {
        let struct_info = type_info.as_struct().unwrap();

        // Copy the contents of the struct based on what kind of pointer we are dealing with
        let gc_handle = if struct_info.memory_kind == abi::StructMemoryKind::Value {
            // For a value struct, `ptr` points to a struct value.

            // Create a new object using the runtime's intrinsic
            let mut gc_handle = runtime.gc().alloc(type_info);

            // Construct
            let src = ptr.cast::<u8>().as_ptr() as *const _;
            let dest = unsafe { gc_handle.deref_mut::<u8>() };
            unsafe { ptr::copy_nonoverlapping(src, dest, type_info.layout().size()) };

            gc_handle
        } else {
            // For a gc struct, `ptr` points to a `GcPtr`.
            unsafe { *ptr.cast::<GcPtr>().as_ptr() }
        };

        StructRef::new(RawStruct(gc_handle), runtime)
    }

    fn marshal_to_ptr(value: Self, mut ptr: NonNull<Self::MunType>, type_info: &Arc<Type>) {
        let struct_info = type_info.as_struct().unwrap();
        if struct_info.memory_kind == abi::StructMemoryKind::Value {
            let dest = ptr.cast::<u8>().as_ptr();
            unsafe {
                ptr::copy_nonoverlapping(
                    value.into_raw().get_ptr(),
                    dest,
                    type_info.layout().size(),
                )
            };
        } else {
            unsafe { *ptr.as_mut() = value.into_raw() };
        }
    }
}

impl<'r> ReturnTypeReflection for StructRef<'r> {
    /// Returns true if this specified type can be stored in an instance of this type
    fn accepts_type(ty: &Arc<Type>) -> bool {
        ty.is_struct()
    }

    fn type_hint() -> &'static str {
        "struct"
    }
}

/// Type-agnostic wrapper for interoperability with a Mun struct, that has been rooted. To marshal,
/// obtain a `StructRef` for the `RootedStruct`.
#[derive(Clone)]
pub struct RootedStruct {
    handle: GcRootPtr,
}

impl RootedStruct {
    /// Creates a `RootedStruct` that wraps a raw Mun struct.
    fn new(gc: &Arc<GarbageCollector>, raw: RawStruct) -> Self {
        assert!(gc.ptr_type(raw.0).is_struct());
        Self {
            handle: GcRootPtr::new(gc, raw.0),
        }
    }

    /// Converts the `RootedStruct` into a `StructRef`, using an external shared reference to a
    /// `Runtime`.
    pub fn as_ref<'r>(&self, runtime: &'r Runtime) -> StructRef<'r> {
        assert_eq!(Arc::as_ptr(&runtime.gc), self.handle.runtime().as_ptr());
        StructRef::new(RawStruct(self.handle.handle()), runtime)
    }
}

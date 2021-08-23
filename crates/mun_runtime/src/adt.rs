use crate::garbage_collector::{GcPtr, GcRootPtr, UnsafeTypeInfo};
use crate::{
    marshal::Marshal,
    reflection::{
        equals_argument_type, equals_return_type, ArgumentReflection, ReturnTypeReflection,
    },
    Runtime,
};
use memory::gc::{GcRuntime, HasIndirectionPtr};
use once_cell::sync::OnceCell;
use std::cell::{Ref, RefCell};
use std::{
    marker::PhantomPinned,
    mem::MaybeUninit,
    pin::Pin,
    ptr::{self, NonNull},
    rc::Rc,
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
    pub fn root(self, runtime: Rc<RefCell<Runtime>>) -> RootedStruct {
        RootedStruct::new(&self.runtime.gc, runtime, self.raw)
    }

    /// Returns the type information of the struct.
    pub fn type_info(&self) -> &abi::TypeInfo {
        // Safety: The type returned from `ptr_type` is guaranteed to live at least as long as
        // `Runtime` does not change. As the lifetime of `TypeInfo` is tied to the lifetime of
        // `Runtime`, this is safe.
        unsafe { &*self.runtime.gc.ptr_type(self.raw.0).into_inner().as_ptr() }
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
        // Safety: self.raw's memory pointer is never null
        NonNull::new_unchecked(self.raw.get_ptr().add(offset as usize).cast::<T>() as *mut _)
    }

    /// Retrieves the value of the field corresponding to the specified `field_name`.
    pub fn get<T: ReturnTypeReflection + Marshal<'s>>(&self, field_name: &str) -> Result<T, String>
    where
        T: 's,
    {
        let type_info = self.type_info();

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
            unsafe { self.field_offset_unchecked::<T::MunType>(struct_info, field_idx) };
        Ok(Marshal::marshal_from_ptr(
            field_ptr,
            self.runtime,
            Some(field_type),
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
        let field_idx =
            abi::StructInfo::find_field_index(type_info.name(), struct_info, field_name)?;

        // Safety: If we found the `field_idx`, we are guaranteed to also have the `field_type` and
        // `field_offset`.
        let field_type = unsafe { struct_info.field_types().get_unchecked(field_idx) };
        equals_argument_type(self.runtime, field_type, &value).map_err(|(expected, found)| {
            format!(
                "Mismatched types for `{}::{}`. Expected: `{}`. Found: `{}`.",
                type_info.name(),
                field_name,
                expected,
                found,
            )
        })?;

        let field_ptr =
            unsafe { self.field_offset_unchecked::<T::MunType>(struct_info, field_idx) };
        let old = Marshal::marshal_from_ptr(field_ptr, self.runtime, Some(field_type));
        Marshal::marshal_to_ptr(value, field_ptr, Some(field_type));
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
        let field_idx =
            abi::StructInfo::find_field_index(type_info.name(), struct_info, field_name)?;

        // Safety: If we found the `field_idx`, we are guaranteed to also have the `field_type` and
        // `field_offset`.
        let field_type = unsafe { struct_info.field_types().get_unchecked(field_idx) };
        equals_argument_type(self.runtime, field_type, &value).map_err(|(expected, found)| {
            format!(
                "Mismatched types for `{}::{}`. Expected: `{}`. Found: `{}`.",
                type_info.name(),
                field_name,
                expected,
                found,
            )
        })?;

        let field_ptr =
            unsafe { self.field_offset_unchecked::<T::MunType>(struct_info, field_idx) };
        Marshal::marshal_to_ptr(value, field_ptr, Some(field_type));
        Ok(())
    }
}

impl<'r> ArgumentReflection for StructRef<'r> {
    fn type_guid(&self, runtime: &Runtime) -> abi::Guid {
        // Safety: The type returned from `ptr_type` is guaranteed to live at least as long as
        // `Runtime` does not change. As we hold a shared reference to `Runtime`, this is safe.
        unsafe { runtime.gc().ptr_type(self.raw.0).into_inner().as_ref().guid }
    }

    fn type_name(&self, runtime: &Runtime) -> &str {
        // Safety: The type returned from `ptr_type` is guaranteed to live at least as long as
        // `Runtime` does not change. As we hold a shared reference to `Runtime`, this is safe.
        unsafe { (&*runtime.gc().ptr_type(self.raw.0).into_inner().as_ptr()).name() }
    }
}

impl<'r> ReturnTypeReflection for StructRef<'r> {
    fn type_name() -> &'static str {
        "struct"
    }

    fn type_guid() -> abi::Guid {
        // TODO: Once `const_fn` lands, replace this with a const md5 hash
        static GUID: OnceCell<abi::Guid> = OnceCell::new();
        *GUID.get_or_init(|| abi::Guid(md5::compute(<Self as ReturnTypeReflection>::type_name()).0))
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
        type_info: Option<&abi::TypeInfo>,
    ) -> StructRef<'s>
    where
        Self: 's,
        'r: 's,
    {
        // Safety: `type_info` is only `None` for the `()` type
        let type_info = type_info.unwrap();
        let struct_info = type_info.as_struct().unwrap();

        // Copy the contents of the struct based on what kind of pointer we are dealing with
        let gc_handle = if struct_info.memory_kind == abi::StructMemoryKind::Value {
            // For a value struct, `ptr` points to a struct value.

            // Create a new object using the runtime's intrinsic
            let mut gc_handle = {
                runtime.gc().alloc(
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

        StructRef::new(RawStruct(gc_handle), runtime)
    }

    fn marshal_to_ptr(
        value: Self,
        mut ptr: NonNull<Self::MunType>,
        type_info: Option<&abi::TypeInfo>,
    ) {
        // `type_info` is only `None` for the `()` type
        let type_info = type_info.unwrap();

        let struct_info = type_info.as_struct().unwrap();
        if struct_info.memory_kind == abi::StructMemoryKind::Value {
            let dest = ptr.cast::<u8>().as_ptr();
            let size = type_info.size_in_bytes();
            unsafe { ptr::copy_nonoverlapping(value.into_raw().get_ptr(), dest, size as usize) };
        } else {
            unsafe { *ptr.as_mut() = value.into_raw() };
        }
    }
}

/// Type-agnostic wrapper for interoperability with a Mun struct, that has been rooted. To marshal,
/// obtain a `StructRef` for the `RootedStruct`.
#[derive(Clone)]
pub struct RootedStruct {
    handle: GcRootPtr,
    runtime: Rc<RefCell<Runtime>>,
}

impl RootedStruct {
    /// Creates a `RootedStruct` that wraps a raw Mun struct.
    fn new<G: GcRuntime<UnsafeTypeInfo>>(
        gc: &Arc<G>,
        runtime: Rc<RefCell<Runtime>>,
        raw: RawStruct,
    ) -> Self {
        let handle = {
            let runtime_ref = runtime.borrow();
            // Safety: The type returned from `ptr_type` is guaranteed to live at least as long as
            // `Runtime` does not change. As we hold a shared reference to `Runtime`, this is safe.
            assert!(unsafe { gc.ptr_type(raw.0).into_inner().as_ref().data.is_struct() });

            GcRootPtr::new(&runtime_ref.gc, raw.0)
        };

        Self { handle, runtime }
    }

    /// Converts the `RootedStruct` into a `StructRef`, using an external shared reference to a
    /// `Runtime`.
    ///
    /// # Safety
    ///
    /// The `RootedStruct` should have been allocated by the `Runtime`.
    pub unsafe fn as_ref<'r>(&self, runtime: &'r Runtime) -> StructRef<'r> {
        StructRef::new(RawStruct(self.handle.handle()), runtime)
    }

    /// Converts the `RootedStruct` to a pinned `RootedStructRef` that can be used just like a
    /// `StructRef`.
    pub fn by_ref(&self) -> Pin<Box<RootedStructRef>> {
        RootedStructRef::new(RawStruct(self.handle.handle()), self.borrow_runtime())
    }

    /// Borrows the struct's runtime.
    pub fn borrow_runtime(&self) -> Ref<Runtime> {
        self.runtime.borrow()
    }
}

/// Type-agnostic wrapper for safely obtaining a `StructRef` from a `RootedStruct`.
pub struct RootedStructRef<'s> {
    runtime: Ref<'s, Runtime>,
    struct_ref: MaybeUninit<StructRef<'s>>,
    _pin: PhantomPinned,
}

impl<'s> RootedStructRef<'s> {
    fn new(raw: RawStruct, runtime: Ref<'s, Runtime>) -> Pin<Box<Self>> {
        let struct_ref = RootedStructRef {
            runtime,
            struct_ref: MaybeUninit::uninit(),
            _pin: PhantomPinned,
        };
        let mut boxed = Box::pin(struct_ref);

        let runtime = NonNull::from(&boxed.runtime);

        // Safety: Modifying a field doesn't move the whole struct
        unsafe {
            let struct_ref = StructRef::new(raw, &*runtime.as_ptr());
            let mut_ref: Pin<&mut Self> = Pin::as_mut(&mut boxed);
            Pin::get_unchecked_mut(mut_ref)
                .struct_ref
                .as_mut_ptr()
                .write(struct_ref);
        }

        boxed
    }
}

impl<'s> std::ops::Deref for RootedStructRef<'s> {
    type Target = StructRef<'s>;

    fn deref(&self) -> &Self::Target {
        // Safety: We always guarantee to set the `struct_ref` upon construction.
        unsafe { &*self.struct_ref.as_ptr() }
    }
}

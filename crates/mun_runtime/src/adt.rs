use crate::{
    garbage_collector::GcRootPtr,
    marshal::Marshal,
    reflection::{equals_return_type, ArgumentReflection, ReturnTypeReflection},
    Runtime,
};
use memory::{
    gc::{GcPtr, GcRuntime, HasIndirectionPtr},
    TypeInfo,
};
use once_cell::sync::OnceCell;
use std::{
    cell::{Ref, RefCell},
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
    pub fn type_info(&self) -> Arc<TypeInfo> {
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

        unsafe { NonNull::new_unchecked(ptr.add(offset).cast::<T>() as *mut T) }
    }

    /// Retrieves the value of the field corresponding to the specified `field_name`.
    pub fn get<T: ReturnTypeReflection + Marshal<'s>>(&self, field_name: &str) -> Result<T, String>
    where
        T: 's,
    {
        let type_info = self.type_info();

        // Safety: `as_struct` is guaranteed to return `Some` for `StructRef`s.
        let struct_info = type_info.as_struct().unwrap();

        let (field_type, field_offset) =
            struct_info.find_field_by_name(field_name).ok_or_else(|| {
                format!(
                    "Struct `{}` does not contain field `{}`.",
                    type_info.name, field_name
                )
            })?;

        equals_return_type::<T>(&field_type).map_err(|(expected, found)| {
            format!(
                "Mismatched types for `{}::{}`. Expected: `{}`. Found: `{}`.",
                type_info.name, field_name, expected, found,
            )
        })?;

        // If we found the `field_idx`, we are guaranteed to also have the `field_offset`
        let field_ptr = unsafe { self.get_field_ptr_unchecked::<T::MunType>(field_offset) };
        Ok(Marshal::marshal_from_ptr(
            field_ptr,
            self.runtime,
            field_type,
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

        let (field_type, field_offset) =
            struct_info.find_field_by_name(field_name).ok_or_else(|| {
                format!(
                    "Struct `{}` does not contain field `{}`.",
                    type_info.name, field_name
                )
            })?;

        if field_type.id != value.type_id(self.runtime) {
            return Err(format!(
                "Mismatched types for `{}::{}`. Expected: `{}`. Found: `{}`.",
                type_info.name,
                field_name,
                value.type_name(self.runtime),
                field_type
            ));
        }

        let field_ptr = unsafe { self.get_field_ptr_unchecked::<T::MunType>(field_offset) };
        let old = Marshal::marshal_from_ptr(field_ptr, self.runtime, field_type);
        Marshal::marshal_to_ptr(value, field_ptr, self.runtime, field_type);
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

        let (field_type, field_offset) =
            struct_info.find_field_by_name(field_name).ok_or_else(|| {
                format!(
                    "Struct `{}` does not contain field `{}`.",
                    type_info.name, field_name
                )
            })?;

        if field_type.id != value.type_id(self.runtime) {
            return Err(format!(
                "Mismatched types for `{}::{}`. Expected: `{}`. Found: `{}`.",
                type_info.name,
                field_name,
                value.type_name(self.runtime),
                field_type
            ));
        }

        let field_ptr = unsafe { self.get_field_ptr_unchecked::<T::MunType>(field_offset) };
        Marshal::marshal_to_ptr(value, field_ptr, self.runtime, field_type);
        Ok(())
    }
}

impl<'r> ArgumentReflection for StructRef<'r> {
    fn type_id(&self, runtime: &Runtime) -> abi::TypeId {
        runtime.gc().ptr_type(self.raw.0).id.clone()
    }

    fn type_name(&self, runtime: &Runtime) -> &str {
        &runtime.gc().ptr_type(self.raw.0).name
    }
}

impl<'r> ReturnTypeReflection for StructRef<'r> {
    fn type_name() -> &'static str {
        "struct"
    }

    fn type_id() -> abi::TypeId {
        // TODO: Once `const_fn` lands, replace this with a const md5 hash
        static GUID: OnceCell<abi::TypeId> = OnceCell::new();
        *GUID.get_or_init(|| {
            abi::Guid::from(<Self as ReturnTypeReflection>::type_name().as_bytes()).into()
        })
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
        type_info: &Arc<TypeInfo>,
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
            unsafe { ptr::copy_nonoverlapping(src, dest, type_info.layout.size()) };

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
        runtime: &Runtime,
        type_info: &Arc<TypeInfo>,
    ) {
        let struct_info = type_info.as_struct().unwrap();
        if struct_info.memory_kind == abi::StructMemoryKind::Value {
            let dest = ptr.cast::<u8>().as_ptr();
            unsafe {
                ptr::copy_nonoverlapping(value.into_raw().get_ptr(), dest, type_info.layout.size())
            };
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
    fn new<G: GcRuntime>(gc: &Arc<G>, runtime: Rc<RefCell<Runtime>>, raw: RawStruct) -> Self {
        let handle = {
            assert!(gc.ptr_type(raw.0).data.is_struct());

            let runtime_ref = runtime.borrow();
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

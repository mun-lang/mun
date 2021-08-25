use crate::garbage_collector::{GcPtr, GcRootPtr};
use crate::{ArgumentReflection, Marshal, ReturnTypeReflection, Runtime, UnsafeTypeInfo};
use abi::TypeInfo;
use memory::gc::{GcRuntime, HasIndirectionPtr};
use memory::{ArrayMemoryLayout, ArrayType, CompositeType};
use once_cell::sync::OnceCell;
use std::cell::{Ref, RefCell};
use std::marker::{PhantomData, PhantomPinned};
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::Arc;

/// Represents a Mun array pointer.
#[repr(transparent)]
#[derive(Clone)]
pub struct RawArray(GcPtr);

impl RawArray {
    /// Returns a pointer to the array memory.
    ///
    /// # Safety
    ///
    /// Dereferencing might cause undefined behavior
    pub unsafe fn get_ptr(&self) -> NonNull<u8> {
        self.0.deref()
    }
}

/// Type-agnostic wrapper for interoperability with a Mun array. This is merely a reference to the
/// Mun array, that will be garbage collected unless it is rooted.
#[derive(Clone)]
pub struct ArrayRef<'a, T> {
    raw: RawArray,
    runtime: &'a Runtime,
    _phantom: PhantomData<T>,
}

impl<'array, T: Marshal<'array> + 'array> ArrayRef<'array, T> {
    /// Creates a `ArrayRef` that wraps a raw Mun struct.
    fn new<'runtime>(raw: RawArray, runtime: &'runtime Runtime) -> Self
    where
        'runtime: 'array,
    {
        Self {
            raw,
            runtime,
            _phantom: Default::default(),
        }
    }

    /// Consumes the `ArrayRef`, returning a raw Mun array.
    pub fn into_raw(self) -> RawArray {
        self.raw
    }

    /// Roots the `ArrayRef`.
    pub fn root(self, runtime: Rc<RefCell<Runtime>>) -> RootedArray<T> {
        RootedArray::new(&self.runtime.gc, runtime, self.raw)
    }

    /// Returns the number of elements stored in the array
    pub fn len(&self) -> usize {
        unsafe {
            let value_ty = self.runtime.gc.ptr_type(self.raw.0);
            let array_ty = value_ty
                .as_array()
                .expect("type of the array value must be an array");
            let value_ptr = self.raw.get_ptr();
            array_ty.retrieve_length(value_ptr)
        }
    }

    /// Returns true if this array does not contain a single element.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the length of the array
    pub fn capacity(&self) -> usize {
        unsafe {
            let value_ty = self.runtime.gc.ptr_type(self.raw.0);
            let array_ty = value_ty
                .as_array()
                .expect("type of the array value must be an array");
            let value_ptr = self.raw.get_ptr();
            array_ty.retrieve_capacity(value_ptr)
        }
    }

    /// Returns an iterator to iterate over the elements of the array.
    pub fn iter(&self) -> impl Iterator<Item = T> + 'array
    where
        T: 'array,
    {
        let value_ty = self.runtime.gc.ptr_type(self.raw.0);
        let array_ty = value_ty
            .as_array()
            .expect("type of the array value must be an array");
        let value_ptr = unsafe { self.raw.get_ptr() };
        let iter = unsafe { array_ty.elements(value_ptr) };
        let element_ty = unsafe { array_ty.element_type().into_inner().as_ref() };
        let runtime = self.runtime;
        iter.map(move |element_ptr| {
            T::marshal_from_ptr(element_ptr.cast(), runtime, Some(element_ty))
        })
    }
}

impl<'a, T: Marshal<'a> + ReturnTypeReflection> ReturnTypeReflection for ArrayRef<'a, T> {
    fn type_guid() -> abi::Guid {
        // TODO: Once `const_fn` lands, replace this with a const md5 hash
        static GUID: OnceCell<abi::Guid> = OnceCell::new();
        *GUID.get_or_init(|| abi::Guid(md5::compute(<Self as ReturnTypeReflection>::type_name()).0))
    }

    fn type_name() -> &'static str {
        static NAME: OnceCell<String> = OnceCell::new();
        NAME.get_or_init(|| format!("[{}]", T::type_name()))
    }

    /// Returns true if this type equals the given type information
    fn equals_type(type_info: &abi::TypeInfo) -> bool {
        type_info
            .as_array()
            .map(|arr| T::equals_type(arr.element_type()))
            .unwrap_or(false)
    }
}

impl<'a, T: Marshal<'a> + ArgumentReflection> ArgumentReflection for ArrayRef<'a, T> {
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

impl<'a, T: Marshal<'a> + 'a> Marshal<'a> for ArrayRef<'a, T> {
    type MunType = RawArray;

    fn marshal_from<'runtime>(value: Self::MunType, runtime: &'runtime Runtime) -> Self
    where
        Self: 'a,
        'runtime: 'a,
    {
        ArrayRef::new(value, runtime)
    }

    fn marshal_into(self) -> Self::MunType {
        self.raw
    }

    fn marshal_from_ptr<'runtime>(
        ptr: NonNull<Self::MunType>,
        runtime: &'runtime Runtime,
        _type_info: Option<&TypeInfo>,
    ) -> Self
    where
        Self: 'a,
        'runtime: 'a,
    {
        let handle = unsafe { *ptr.cast::<GcPtr>().as_ptr() };
        ArrayRef::new(RawArray(handle), runtime)
    }

    fn marshal_to_ptr(value: Self, mut ptr: NonNull<Self::MunType>, _type_info: Option<&TypeInfo>) {
        unsafe { *ptr.as_mut() = value.into_raw() };
    }
}

/// Type-agnostic wrapper for interoperability with a Mun struct, that has been rooted. To marshal,
/// obtain a `ArrayRef` for the `RootedArray`.
#[derive(Clone)]
pub struct RootedArray<T> {
    handle: GcRootPtr,
    runtime: Rc<RefCell<Runtime>>,
    _data: PhantomData<T>,
}

impl<T> RootedArray<T> {
    /// Creates a `RootedArray` that wraps a raw Mun struct.
    fn new<G: GcRuntime<UnsafeTypeInfo>>(
        gc: &Arc<G>,
        runtime: Rc<RefCell<Runtime>>,
        raw: RawArray,
    ) -> Self {
        let handle = {
            let runtime_ref = runtime.borrow();
            // Safety: The type returned from `ptr_type` is guaranteed to live at least as long as
            // `Runtime` does not change. As we hold a shared reference to `Runtime`, this is safe.
            assert!(unsafe { gc.ptr_type(raw.0).into_inner().as_ref().data.is_array() });

            GcRootPtr::new(&runtime_ref.gc, raw.0)
        };

        Self {
            handle,
            runtime,
            _data: Default::default(),
        }
    }

    /// Converts the `RootedArray` into a `ArrayRef`, using an external shared reference to a
    /// `Runtime`.
    ///
    /// # Safety
    ///
    /// The `RootedArray` should have been allocated by the `Runtime`.
    pub unsafe fn as_ref<'r>(&self, runtime: &'r Runtime) -> ArrayRef<'r, T>
    where
        T: Marshal<'r>,
        T: 'r,
    {
        ArrayRef::new(RawArray(self.handle.handle()), runtime)
    }

    /// Converts the `RootedArray` to a pinned `RootedArrayRef` that can be used just like a
    /// `ArrayRef`.
    pub fn by_ref<'r>(&'r self) -> Pin<Box<RootedArrayRef<'r, T>>>
    where
        T: Marshal<'r>,
        T: 'r,
    {
        RootedArrayRef::new(RawArray(self.handle.handle()), self.borrow_runtime())
    }

    /// Borrows the struct's runtime.
    pub fn borrow_runtime(&self) -> Ref<Runtime> {
        self.runtime.borrow()
    }
}

/// Type-agnostic wrapper for safely obtaining a `ArrayRef` from a `RootedArray`.
pub struct RootedArrayRef<'s, T> {
    runtime: Ref<'s, Runtime>,
    array_ref: MaybeUninit<ArrayRef<'s, T>>,
    _pin: PhantomPinned,
}

impl<'s, T: Marshal<'s> + 's> RootedArrayRef<'s, T> {
    fn new(raw: RawArray, runtime: Ref<'s, Runtime>) -> Pin<Box<Self>> {
        let array_ref = RootedArrayRef {
            runtime,
            array_ref: MaybeUninit::uninit(),
            _pin: PhantomPinned,
        };
        let mut boxed = Box::pin(array_ref);

        let runtime = NonNull::from(&boxed.runtime);

        // Safety: Modifying a field doesn't move the whole struct
        unsafe {
            let array_ref = ArrayRef::new(raw, &*runtime.as_ptr());
            let mut_ref: Pin<&mut Self> = Pin::as_mut(&mut boxed);
            Pin::get_unchecked_mut(mut_ref)
                .array_ref
                .as_mut_ptr()
                .write(array_ref);
        }

        boxed
    }
}

impl<'s, T> std::ops::Deref for RootedArrayRef<'s, T> {
    type Target = ArrayRef<'s, T>;

    fn deref(&self) -> &Self::Target {
        // Safety: We always guarantee to set the `array_ref` upon construction.
        unsafe { &*self.array_ref.as_ptr() }
    }
}

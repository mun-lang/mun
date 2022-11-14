use crate::{
    garbage_collector::GcRootPtr, ArgumentReflection, GarbageCollector, Marshal,
    ReturnTypeReflection, Runtime,
};
use mun_memory::{
    gc::{Array, GcPtr, GcRuntime, HasIndirectionPtr},
    Type,
};
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::sync::Arc;

/// Represents a Mun array pointer.
#[repr(transparent)]
#[derive(Clone)]
pub struct RawArray(pub(crate) GcPtr);

impl RawArray {
    /// Returns a pointer to the array memory.
    ///
    /// # Safety
    ///
    /// Dereferencing might cause undefined behavior
    pub unsafe fn get_ptr(&self) -> *const u8 {
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
    pub(crate) fn new<'runtime>(raw: RawArray, runtime: &'runtime Runtime) -> Self
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
    pub fn root(self) -> RootedArray<T> {
        RootedArray::new(&self.runtime.gc, self.raw)
    }

    /// Returns the type information of the array.
    pub fn type_info(&self) -> Type {
        self.runtime.gc.ptr_type(self.raw.0)
    }

    /// Returns the number of elements stored in the array
    pub fn len(&self) -> usize {
        self.runtime
            .gc
            .as_ref()
            .array(self.raw.0)
            .expect("the internal handle does not refer to an array")
            .length()
    }

    /// Returns true if this array does not contain a single element.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the length of the array
    pub fn capacity(&self) -> usize {
        self.runtime
            .gc
            .as_ref()
            .array(self.raw.0)
            .expect("the internal handle does not refer to an array")
            .capacity()
    }

    /// Returns an iterator to iterate over the elements of the array.
    pub fn iter(&self) -> impl Iterator<Item = T> + 'array
    where
        T: 'array,
    {
        let handle = self
            .runtime
            .gc
            .as_ref()
            .array(self.raw.0)
            .expect("type of the array value must be an array");
        let element_ty = handle.element_type();
        let runtime = self.runtime;
        handle
            .elements()
            .map(move |element_ptr| T::marshal_from_ptr(element_ptr.cast(), runtime, &element_ty))
    }
}

impl<'a, T: Marshal<'a> + ReturnTypeReflection> ReturnTypeReflection for ArrayRef<'a, T> {
    fn accepts_type(ty: &Type) -> bool {
        if let Some(arr) = ty.as_array() {
            T::accepts_type(&arr.element_type())
        } else {
            false
        }
    }

    fn type_hint() -> &'static str {
        // TODO: Improve this
        "array"
    }
}

impl<'a, T: Marshal<'a> + ArgumentReflection + 'a> ArgumentReflection for ArrayRef<'a, T> {
    fn type_info(&self, _runtime: &Runtime) -> Type {
        self.type_info()
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
        _type_info: &Type,
    ) -> Self
    where
        Self: 'a,
        'runtime: 'a,
    {
        let handle = unsafe { *ptr.cast::<GcPtr>().as_ptr() };
        ArrayRef::new(RawArray(handle), runtime)
    }

    fn marshal_to_ptr(value: Self, mut ptr: NonNull<Self::MunType>, _type_info: &Type) {
        unsafe { *ptr.as_mut() = value.into_raw() };
    }
}

/// Type-agnostic wrapper for interoperability with a Mun struct, that has been rooted. To marshal,
/// obtain a `ArrayRef` for the `RootedArray`.
#[derive(Clone)]
pub struct RootedArray<T> {
    handle: GcRootPtr,
    _data: PhantomData<T>,
}

impl<T> RootedArray<T> {
    /// Creates a `RootedArray` that wraps a raw Mun struct.
    fn new(gc: &Arc<GarbageCollector>, raw: RawArray) -> Self {
        assert!(gc.ptr_type(raw.0).is_array());
        Self {
            handle: GcRootPtr::new(gc, raw.0),
            _data: Default::default(),
        }
    }

    /// Converts the `RootedArray` into an `ArrayRef<T>`, using an external shared reference to a
    /// `Runtime`.
    pub fn as_ref<'r>(&self, runtime: &'r Runtime) -> ArrayRef<'r, T>
    where
        T: Marshal<'r> + 'r,
    {
        assert_eq!(Arc::as_ptr(&runtime.gc), self.handle.runtime().as_ptr());
        ArrayRef::new(RawArray(self.handle.handle()), runtime)
    }
}

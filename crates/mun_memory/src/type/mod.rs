//! Type information in Mun is stored globally. This allows type information to
//! be stored easily.
//!
//! A type is referred to with [`Type`]. A `Type` holds a reference to the
//! underlying data which is managed by the runtime. `Type`s can be freely
//! created through a [`StructTypeBuilder`], via [`HasStaticType`] or by
//! querying other types ([`Type::pointer_type`] for instance). Cloning a
//! [`Type`] is a cheap operation.
//!
//! Type information is stored globally on the heap and is freed when no longer
//! referenced by a `Type`. However, since `Type`s can reference each other a
//! garbage collection algorithm is used to clean up unreferenced type
//! information. See [`Type::collect_unreferenced_types()`].

pub mod ffi;

use std::{
    alloc::Layout,
    borrow::Cow,
    collections::VecDeque,
    ffi::c_void,
    fmt::{self, Debug, Display, Formatter},
    hash::{Hash, Hasher},
    ops::Deref,
    ptr::NonNull,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Once,
    },
};

use itertools::izip;
use mun_abi::{self as abi, static_type_map::StaticTypeMap};
use once_cell::sync::Lazy;
use parking_lot::{lock_api::MutexGuard, Mutex, RawMutex, RwLock};

use crate::{type_table::TypeTable, TryFromAbiError};

static GLOBAL_TYPE_STORE: Lazy<Arc<TypeDataStore>> = Lazy::new(Default::default);

/// A type store holds a list of interconnected [`TypeData`]s. Type information
/// can contain cycles so the `TypeData`s refer to each other via pointers. The
/// `TypeDataStore` owns the heap allocated `TypeData` instances.
///
/// By calling [`TypeDataStore::collect_garbage`] types that are no longer
/// referenced by [`Type`]s are removed.
#[derive(Default)]
struct TypeDataStore {
    types: Mutex<VecDeque<Box<TypeData>>>,
}

/// Result information after a call to [`TypeDataStore::collect_garbage`].
#[derive(Clone, Debug)]
pub struct TypeCollectionStats {
    pub collected_types: usize,
    pub remaining_types: usize,
}

/// A status stored with every [`TypeData`] which stores the current usage of
/// the `TypeData`. Initially, types are marked as `Initializing` which
/// indicates that they should not 'yet' participate in garbage-collection.
#[derive(Eq, PartialEq)]
enum Mark {
    Used,
    Unused,
    Initializing,
}

impl TypeDataStore {
    /// Called to collect types that are no longer externally referenced.
    pub fn collect_garbage(&self) -> TypeCollectionStats {
        let mut lock = self.types.lock();

        // Reset all mark flags.
        let mut queue = VecDeque::new();
        for ty in lock.iter_mut() {
            if ty.mark != Mark::Initializing {
                if ty.external_references.load(Ordering::Acquire) > 0 {
                    ty.mark = Mark::Used;
                    queue.push_back(unsafe {
                        NonNull::new_unchecked(Box::as_mut(ty) as *mut TypeData)
                    });
                } else {
                    ty.mark = Mark::Unused;
                };
            }
        }

        // Trace all types
        while let Some(ty) = queue.pop_back() {
            let ty = unsafe { ty.as_ref() };
            match &ty.data {
                TypeDataKind::Struct(s) => {
                    for field in s.fields.iter() {
                        let mut field_ty = field.type_info;
                        let field_ty = unsafe { field_ty.as_mut() };
                        if field_ty.mark == Mark::Unused {
                            field_ty.mark = Mark::Used;
                            queue.push_back(field.type_info);
                        }
                    }
                }
                TypeDataKind::Pointer(p) => {
                    let mut pointee = p.pointee;
                    let pointee = unsafe { pointee.as_mut() };
                    if pointee.mark == Mark::Unused {
                        pointee.mark = Mark::Used;
                        queue.push_back(p.pointee);
                    }
                }
                TypeDataKind::Array(a) => {
                    let mut element_ty = a.element_ty;
                    let element_ty = unsafe { element_ty.as_mut() };
                    if element_ty.mark == Mark::Unused {
                        element_ty.mark = Mark::Used;
                        queue.push_back(a.element_ty);
                    }
                }
                TypeDataKind::Primitive(_) | TypeDataKind::Uninitialized => {}
            }

            // Iterate over the indirections. This is an interesting case safety wise,
            // because at this very moment another thread might be accessing
            // this as well. However this is safe because we use `allocate_into`
            // to allocate the values.
            for indirection in [
                &ty.mutable_pointer_type,
                &ty.immutable_pointer_type,
                &ty.array_type,
            ] {
                let read_lock = indirection.read();
                if let &Some(mut indirection_ref) = &*read_lock {
                    let reference = unsafe { indirection_ref.as_mut() };
                    if reference.mark == Mark::Unused {
                        reference.mark = Mark::Used;
                        queue.push_back(indirection_ref);
                    }
                }
            }
        }

        // Iterate over all objects and remove the ones that are no longer referenced
        let mut types_removed = 0;
        let mut index = 0;
        while index < lock.len() {
            let ty = &(&*lock)[index];
            if ty.mark == Mark::Unused {
                lock.swap_remove_back(index);
                types_removed += 1;
            } else {
                index += 1;
            }
        }

        TypeCollectionStats {
            collected_types: types_removed,
            remaining_types: lock.len(),
        }
    }

    /// Tries to convert multiple [`abi::TypeDefinition`] to internal type
    /// representations. If the conversion succeeds an updated [`TypeTable`]
    /// is returned
    pub fn try_from_abi<'abi>(
        self: &Arc<Self>,
        definitions: impl Iterator<Item = &'abi abi::TypeDefinition<'abi>>,
        mut type_table: TypeTable,
    ) -> Result<(TypeTable, Vec<Type>), TryFromAbiError<'abi>> {
        // Acquire a lock in the type entries
        let mut entries = self.types.lock();

        // Create uninitialized types for all the definitions
        let mut types = Vec::new();
        let mut definition_and_type = Vec::with_capacity(definitions.size_hint().0);
        for type_def in definitions {
            let ty = self.allocate_inner(
                type_def.name().to_owned(),
                Layout::from_size_align(type_def.size_in_bytes(), type_def.alignment())
                    .expect("invalid abi type definition layout"),
                TypeDataKind::Uninitialized,
                &mut entries,
            );
            type_table.insert_concrete_type(*type_def.as_concrete(), ty.clone());
            types.push(ty.clone());
            definition_and_type.push((type_def, ty));
        }

        std::mem::drop(entries);

        // Next, initialize the types.
        for (type_def, mut ty) in definition_and_type {
            // Safety: we are modifying the inner data of the type here. At this point this
            // is safe because the type cannot be used by anything else yet.
            let inner_ty = unsafe { ty.inner.as_mut() };
            let type_data = match &type_def.data {
                abi::TypeDefinitionData::Struct(s) => {
                    StructData::try_from_abi(s, &type_table)?.into()
                }
            };
            inner_ty.data = type_data;

            // Mark the entry as used. This should be safe because the `type_table` also
            // still holds a strong reference to the type. After that type is
            // potentially dropped (after this function returns) all values has
            // already been initialized.
            inner_ty.mark = Mark::Used;
        }

        Ok((type_table, types))
    }

    fn allocate_inner(
        self: &Arc<Self>,
        name: impl Into<String>,
        layout: Layout,
        data: TypeDataKind,
        entries: &mut MutexGuard<'_, RawMutex, VecDeque<Box<TypeData>>>,
    ) -> Type {
        entries.push_back(Box::new(TypeData {
            name: name.into(),
            layout,
            data,
            external_references: AtomicUsize::new(0),
            immutable_pointer_type: RwLock::default(),
            mutable_pointer_type: RwLock::default(),
            array_type: RwLock::default(),
            mark: Mark::Initializing,
        }));

        // Safety: get a TypeInner with a 'static lifetime. This is safe because of the
        // nature of `Type`. The `Type` struct ensures that the pointed to value
        // is never destructed as long as it lives.
        let entry = unsafe {
            NonNull::new_unchecked(
                &**entries.back().expect("didnt insert") as *const TypeData as *mut _
            )
        };

        // Safety: this operation is safe, because we currently own the instance and the
        // lock.
        unsafe { Type::new_unchecked(entry, self.clone()) }
    }

    /// Allocates a new type instance
    pub fn allocate(
        self: &Arc<Self>,
        name: impl Into<String>,
        layout: Layout,
        data: TypeDataKind,
    ) -> Type {
        let mut entries = self.types.lock();
        let mut ty = self.allocate_inner(name, layout, data, &mut entries);
        unsafe { ty.inner.as_mut() }.mark = Mark::Used;
        ty
    }

    /// Allocates a new type instance but keeps it in an uninitialized state.
    pub fn allocate_uninitialized(
        self: &Arc<Self>,
        name: impl Into<String>,
        layout: Layout,
        data: TypeDataKind,
    ) -> Type {
        let mut entries = self.types.lock();
        self.allocate_inner(name, layout, data, &mut entries)
    }
}

/// A reference to internally stored type information. A `Type` can be used to
/// query information, construct other types, or store type information for
/// later use.
pub struct Type {
    inner: NonNull<TypeData>,

    /// A [`Type`] holds a strong reference to its data store. This ensure that
    /// the data is never deleted before this instance is destroyed.
    ///
    /// This has to be here because we store [`Type`] instances globally both in
    /// Rust and potentially als over FFI. Since the static destruction
    /// order is not completely guarenteed the store might be deallocated
    /// before the last type is deallocated. Keeping a reference to
    /// the store in this instance ensures that the data is kept alive until the
    /// last `Type` is dropped.
    store: Arc<TypeDataStore>,
}

impl Type {
    /// Constructs a new instance from a pointer and the store it belongs to.
    ///
    /// # Safety
    ///
    /// The pointer pointed to by `inner` might be invalid, in which case this
    /// method will cause undefined behavior.
    unsafe fn new_unchecked(mut inner: NonNull<TypeData>, store: Arc<TypeDataStore>) -> Self {
        // Increment the external reference count
        inner
            .as_mut()
            .external_references
            .fetch_add(1, Ordering::AcqRel);

        Self { inner, store }
    }
}

// Types can safely be used across multiple threads.
unsafe impl Send for Type {}
unsafe impl Sync for Type {}

impl Debug for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.kind() {
            TypeKind::Primitive(_) => std::fmt::Display::fmt(self.name(), f),
            TypeKind::Struct(s) => std::fmt::Display::fmt(&s, f),
            TypeKind::Pointer(p) => std::fmt::Display::fmt(&p, f),
            TypeKind::Array(a) => std::fmt::Display::fmt(&a, f),
        }
    }
}

impl Clone for Type {
    fn clone(&self) -> Self {
        self.inner()
            .external_references
            .fetch_add(1, Ordering::AcqRel);

        Self {
            store: self.store.clone(),
            inner: self.inner,
        }
    }
}

impl Drop for Type {
    fn drop(&mut self) {
        self.inner()
            .external_references
            .fetch_sub(1, Ordering::Release);
    }
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        self.inner() == other.inner()
    }
}

impl Eq for Type {}
impl Hash for Type {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner().hash(state);
    }
}

/// Stores type information for a particular type as well as references to
/// related types.
pub struct TypeData {
    /// Type name
    name: String,

    /// The memory layout of the type
    layout: Layout,

    /// Type group
    data: TypeDataKind,

    /// Holds the number of external (non-cyclic) references. Basically the
    /// number of [`Type`] instances pointing to this instance.
    ///
    /// Note that if this ever reaches zero is doesn't mean it's no longer used
    /// because it can still be referenced by other types.
    external_references: AtomicUsize,

    /// The type of an immutable pointer to this type
    immutable_pointer_type: RwLock<Option<NonNull<TypeData>>>,

    /// The type of a mutable pointer to this type
    mutable_pointer_type: RwLock<Option<NonNull<TypeData>>>,

    /// The type of an array of this type
    array_type: RwLock<Option<NonNull<TypeData>>>,

    /// The state of instance with regards to its usage.
    mark: Mark,
}

impl TypeData {
    /// Returns the type that represents a pointer to this type
    fn pointer_type(&self, mutable: bool, store: &Arc<TypeDataStore>) -> Type {
        let cache_key = if mutable {
            &self.mutable_pointer_type
        } else {
            &self.immutable_pointer_type
        };

        {
            let read_lock = cache_key.read();

            // Fast path, the type already exists, return it immediately.
            if let Some(ty) = read_lock.deref().as_ref() {
                return Type {
                    inner: *ty,
                    store: store.clone(),
                };
            }
        }

        // No type is currently stored, allocate a new one.
        let mut ty = store.allocate_uninitialized(
            format!("*{} {}", if mutable { "mut" } else { "const" }, self.name),
            Layout::new::<*const std::ffi::c_void>(),
            PointerData {
                pointee: self.into(),
                mutable,
            }
            .into(),
        );

        // Acquire the write lock
        let mut write_lock = cache_key.write();

        // Get the reference to the inner data, we need this to mark it properly.
        let inner = unsafe { ty.inner.as_mut() };

        // Recheck if another thread acquired the write lock in the mean time
        if let Some(element_ty) = &*write_lock {
            inner.mark = Mark::Used;
            return Type {
                inner: *element_ty,
                store: store.clone(),
            };
        }

        // We store the reference to the array type in the current type. After which we
        // mark the type as used. This ensures that the garbage collector never
        // removes the type from under our noses.
        *write_lock = Some(ty.inner);
        inner.mark = Mark::Used;

        ty
    }

    /// Returns the type that represents a pointer to this type
    fn array_type(&self, store: &Arc<TypeDataStore>) -> Type {
        let cache_key = &self.array_type;

        {
            let read_lock = cache_key.read();

            // Fast path, the type already exists, return it immediately.
            if let Some(ty) = read_lock.deref().as_ref() {
                return Type {
                    inner: *ty,
                    store: store.clone(),
                };
            }
        }

        // No type is currently stored, allocate a new one.
        let mut ty = store.allocate_uninitialized(
            format!("[{}]", self.name),
            Layout::new::<*const std::ffi::c_void>(),
            ArrayData {
                element_ty: self.into(),
            }
            .into(),
        );

        // Acquire the write lock
        let mut write_lock = cache_key.write();

        // Get the reference to the inner data, we need this to mark it properly.
        let inner = unsafe { ty.inner.as_mut() };

        // Recheck if another thread acquired the write lock in the mean time
        if let Some(element_ty) = &*write_lock {
            inner.mark = Mark::Used;
            return Type {
                inner: *element_ty,
                store: store.clone(),
            };
        }

        // We store the reference to the array type in the current type. After which we
        // mark the type as used. This ensures that the garbage collector never
        // removes the type from under our noses.
        *write_lock = Some(ty.inner);
        inner.mark = Mark::Used;

        ty
    }
}

impl PartialEq for TypeData {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.layout == other.layout && self.data == other.data
    }
}

impl Eq for TypeData {}

unsafe impl Send for TypeData {}
unsafe impl Sync for TypeData {}

/// A linked version of [`mun_abi::TypeInfoData`] that has resolved all
/// occurrences of `TypeId` with `TypeInfo`.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum TypeDataKind {
    /// Primitive types (i.e. `()`, `bool`, `float`, `int`, etc.)
    Primitive(abi::Guid),
    /// Struct types (i.e. record, tuple, or unit structs)
    Struct(StructData),
    /// A pointer to another type
    Pointer(PointerData),
    /// An array
    Array(ArrayData),
    /// Indicates that the type has been allocated but it has not yet been
    /// initialized, this indicates that it still needs to be properly
    /// initialized.
    Uninitialized,
}

#[derive(Copy, Clone)]
pub enum TypeKind<'t> {
    /// Primitive types (i.e. `()`, `bool`, `float`, `int`, etc.)
    Primitive(&'t abi::Guid),
    /// Struct types (i.e. record, tuple, or unit structs)
    Struct(StructType<'t>),
    /// A pointer to another type
    Pointer(PointerType<'t>),
    /// An array of values
    Array(ArrayType<'t>),
}

/// A linked version of [`mun_abi::StructInfo`] that has resolved all
/// occurrences of `TypeId` with `TypeInfo`.
#[derive(Clone, Debug)]
struct StructData {
    /// The unique identifier of this struct
    pub guid: abi::Guid,
    /// Struct fields
    pub fields: Vec<FieldData>,
    /// Struct memory kind
    pub memory_kind: abi::StructMemoryKind,
}

/// Reference information of a struct
#[repr(C)]
#[derive(Copy, Clone)]
pub struct StructType<'t> {
    inner: &'t StructData,
    store: &'t Arc<TypeDataStore>,
}

impl<'t> StructType<'t> {
    /// Returns the unique identifier of this struct
    pub fn guid<'s>(&'s self) -> &'t abi::Guid
    where
        't: 's,
    {
        &self.inner.guid
    }

    /// Returns the memory type of this struct
    pub fn memory_kind(&self) -> abi::StructMemoryKind {
        self.inner.memory_kind
    }

    /// Returns true if this struct is a value struct. Value structs are passed
    /// by value and are not allocated by the garbage collector.
    pub fn is_value_struct(&self) -> bool {
        self.memory_kind() == abi::StructMemoryKind::Value
    }

    /// Returns true if this struct is a garbage collected struct.
    pub fn is_gc_struct(&self) -> bool {
        self.memory_kind() == abi::StructMemoryKind::Gc
    }

    /// Returns an iterator over all fields
    pub fn fields(&self) -> Fields<'t> {
        Fields {
            inner: self.inner,
            store: self.store,
        }
    }
}

impl<'t> Display for StructType<'t> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "struct({}) {{",
            if self.is_gc_struct() { "gc" } else { "value" },
        ))?;
        self.fields().iter().try_for_each(|field| {
            f.write_fmt(format_args!("{}: {}, ", field.name(), field.ty()))
        })?;
        f.write_str("}")
    }
}

/// A collection of fields of a struct
#[derive(Copy, Clone)]
pub struct Fields<'t> {
    inner: &'t StructData,
    store: &'t Arc<TypeDataStore>,
}

impl<'t> Fields<'t> {
    /// Returns the number of fields in the struct
    pub fn len(&self) -> usize {
        self.inner.fields.len()
    }

    /// Returns the field at the given index, or `None` if `index` exceeds the
    /// number of fields.
    pub fn get(&self, index: usize) -> Option<Field<'t>> {
        self.inner.fields.get(index).map(|field| Field {
            inner: field,
            store: self.store,
        })
    }

    /// Returns the field with the given name, or `None` if no such field
    /// exists.
    pub fn find_by_name(&self, name: impl AsRef<str>) -> Option<Field<'t>> {
        let field_name = name.as_ref();
        self.iter().find(|field| field.name() == field_name)
    }

    /// Returns an iterator over all fields
    pub fn iter(&self) -> FieldsIterator<'t> {
        FieldsIterator {
            iter: self.inner.fields.iter(),
            store: self.store,
        }
    }
}

impl<'t> IntoIterator for Fields<'t> {
    type Item = Field<'t>;
    type IntoIter = FieldsIterator<'t>;

    fn into_iter(self) -> Self::IntoIter {
        FieldsIterator {
            iter: self.inner.fields.iter(),
            store: self.store,
        }
    }
}

pub struct FieldsIterator<'t> {
    iter: std::slice::Iter<'t, FieldData>,
    store: &'t Arc<TypeDataStore>,
}

impl<'t> Iterator for FieldsIterator<'t> {
    type Item = Field<'t>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|field| Field {
            inner: field,
            store: self.store,
        })
    }
}

/// A linked version of [`mun_abi::PointerInfo`] that has resolved all
/// occurrences of `TypeId` with `TypeInfo`.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct PointerData {
    /// The type to which is pointed
    pub pointee: NonNull<TypeData>,
    /// Whether or not the pointer is mutable
    pub mutable: bool,
}

/// Reference information of a pointer
#[derive(Copy, Clone)]
pub struct PointerType<'t> {
    inner: &'t PointerData,
    store: &'t Arc<TypeDataStore>,
}

impl<'t> PointerType<'t> {
    /// Returns the type to which this pointer points
    pub fn pointee(&self) -> Type {
        // Safety: this operation is safe due to the lifetime constraints on this type
        unsafe { Type::new_unchecked(self.inner.pointee, self.store.clone()) }
    }

    /// Returns true if this is a mutable pointer type
    pub fn is_mutable(&self) -> bool {
        self.inner.mutable
    }

    /// Returns true if this is a immutable pointer type
    pub fn is_immutable(&self) -> bool {
        !self.inner.mutable
    }
}

impl<'t> Display for PointerType<'t> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "*{} {}",
            if self.is_mutable() { "mut" } else { "const" },
            self.pointee()
        ))
    }
}

impl Hash for StructData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.guid.hash(state);
    }
}

impl PartialEq for StructData {
    fn eq(&self, other: &Self) -> bool {
        self.guid == other.guid
    }
}
impl Eq for StructData {}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct ArrayData {
    pub element_ty: NonNull<TypeData>,
}

/// Reference information of an array
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ArrayType<'t> {
    inner: &'t ArrayData,
    store: &'t Arc<TypeDataStore>,
}

impl<'t> ArrayType<'t> {
    /// Returns the type of elements this array stores
    pub fn element_type(&self) -> Type {
        // Safety: this operation is safe due to the lifetime constraints on this type
        unsafe { Type::new_unchecked(self.inner.element_ty, self.store.clone()) }
    }
}

impl<'t> Display for ArrayType<'t> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("[")?;
        std::fmt::Display::fmt(&self.element_type(), f)?;
        f.write_str("]")
    }
}

impl From<StructData> for TypeDataKind {
    fn from(s: StructData) -> Self {
        TypeDataKind::Struct(s)
    }
}

impl From<PointerData> for TypeDataKind {
    fn from(p: PointerData) -> Self {
        TypeDataKind::Pointer(p)
    }
}

impl From<ArrayData> for TypeDataKind {
    fn from(a: ArrayData) -> Self {
        TypeDataKind::Array(a)
    }
}

impl Hash for TypeData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&self.data, state);
    }
}

impl Type {
    /// Collects all data related to types that are no longer referenced by a
    /// [`Type`]. Returns the number of types that were removed.
    pub fn collect_unreferenced_type_data() -> TypeCollectionStats {
        GLOBAL_TYPE_STORE.collect_garbage()
    }

    /// Constructs a new struct type
    pub fn new_struct(
        name: impl Into<String>,
        layout: Layout,
        guid: abi::Guid,
        fields: impl IntoIterator<Item = (String, Type, u16)>,
        memory_kind: abi::StructMemoryKind,
    ) -> Type {
        let fields = fields
            .into_iter()
            .map(|(name, ty, offset)| FieldData {
                name,
                type_info: ty.inner,
                offset,
            })
            .collect::<Vec<_>>();
        GLOBAL_TYPE_STORE.allocate(
            name,
            layout,
            StructData {
                guid,
                fields,
                memory_kind,
            }
            .into(),
        )
    }

    /// Returns a reference to the [`TypeInner`]
    fn inner(&self) -> &TypeData {
        // Safety: taking the reference is always ok because the garbage collector
        // ensures that as long as self (Type) exists the inner stays alive.
        unsafe { self.inner.as_ref() }
    }

    /// Returns the name of the type
    pub fn name(&self) -> &str {
        self.inner().name.as_str()
    }

    /// Returns the memory layout of the data of the type. This is the layout of
    /// the memory when stored on the stack or in the heap.
    pub fn value_layout(&self) -> Layout {
        self.inner().layout
    }

    /// Returns the layout of the type when being referenced.
    pub fn reference_layout(&self) -> Layout {
        if self.is_reference_type() {
            // Reference types are always stored as pointers to an GC object
            Layout::new::<*const c_void>()
        } else {
            self.value_layout()
        }
    }

    /// Returns true if the type is a reference type. Variables of reference
    /// types store references to their data (objects), while variables of
    /// value types directly contain their data.
    pub fn is_reference_type(&self) -> bool {
        match self.kind() {
            TypeKind::Primitive(_) | TypeKind::Pointer(_) => false,
            TypeKind::Array(_) => true,
            TypeKind::Struct(s) => s.is_gc_struct(),
        }
    }

    /// Returns true if the type is a value type. Variables of reference types
    /// store references to their data (objects), while variables of value
    /// types directly contain their data.
    pub fn is_value_type(&self) -> bool {
        match self.kind() {
            TypeKind::Primitive(_) | TypeKind::Pointer(_) => true,
            TypeKind::Array(_) => false,
            TypeKind::Struct(s) => s.is_value_struct(),
        }
    }

    /// Returns true if this instance represents the `TypeInfo` of the given
    /// type.
    ///
    /// ```rust
    /// # use mun_memory::HasStaticType;
    /// assert!(i64::type_info().equals::<i64>());
    /// assert!(!i64::type_info().equals::<f64>())
    /// ```
    pub fn equals<T: HasStaticType>(&self) -> bool {
        T::type_info() == self
    }

    /// Returns whether this is a fundamental type.
    pub fn is_primitive(&self) -> bool {
        matches!(self.kind(), TypeKind::Primitive(_))
    }

    /// Returns whether this is a struct type.
    pub fn is_struct(&self) -> bool {
        matches!(self.kind(), TypeKind::Struct(_))
    }

    /// Returns whether this is a pointer type.
    pub fn is_pointer(&self) -> bool {
        matches!(self.kind(), TypeKind::Pointer(_))
    }

    /// Returns whether this is an array type.
    pub fn is_array(&self) -> bool {
        matches!(self.kind(), TypeKind::Array(_))
    }

    /// Returns the kind of the type
    pub fn kind(&self) -> TypeKind<'_> {
        match &self.inner().data {
            TypeDataKind::Primitive(guid) => TypeKind::Primitive(guid),
            TypeDataKind::Struct(s) => TypeKind::Struct(StructType {
                inner: s,
                store: &self.store,
            }),
            TypeDataKind::Pointer(p) => TypeKind::Pointer(PointerType {
                inner: p,
                store: &self.store,
            }),
            TypeDataKind::Array(a) => TypeKind::Array(ArrayType {
                inner: a,
                store: &self.store,
            }),
            TypeDataKind::Uninitialized => {
                unreachable!("should never be able to query the kind of an uninitialized type")
            }
        }
    }

    /// Returns true if this type is a concrete type. This is the case for any
    /// type that doesn't refer to another type like a pointer.
    pub fn is_concrete(&self) -> bool {
        match self.kind() {
            TypeKind::Primitive(_) | TypeKind::Struct(_) => true,
            TypeKind::Pointer(_) | TypeKind::Array(_) => false,
        }
    }

    /// Returns the GUID associated with this instance if this instance
    /// represents a concrete type.
    pub fn as_concrete(&self) -> Option<&abi::Guid> {
        match self.kind() {
            TypeKind::Primitive(g) => Some(g),
            TypeKind::Struct(s) => Some(s.guid()),
            TypeKind::Pointer(_) | TypeKind::Array(_) => None,
        }
    }

    /// Retrieves the type's struct information, if available.
    pub fn as_struct(&self) -> Option<StructType<'_>> {
        if let TypeKind::Struct(s) = self.kind() {
            Some(s)
        } else {
            None
        }
    }

    /// Retrieves the type's pointer information, if available.
    pub fn as_pointer(&self) -> Option<PointerType<'_>> {
        if let TypeKind::Pointer(p) = self.kind() {
            Some(p)
        } else {
            None
        }
    }

    /// Retrieves the type's array information, if available.
    pub fn as_array(&self) -> Option<ArrayType<'_>> {
        if let TypeKind::Array(a) = self.kind() {
            Some(a)
        } else {
            None
        }
    }

    /// Tries to convert multiple [`abi::TypeDefinition`] to internal type
    /// representations. If the conversion succeeds an updated [`TypeTable`]
    /// is returned.
    pub fn try_from_abi<'abi>(
        type_info: impl IntoIterator<Item = &'abi abi::TypeDefinition<'abi>>,
        type_table: TypeTable,
    ) -> Result<(TypeTable, Vec<Type>), TryFromAbiError<'abi>> {
        GLOBAL_TYPE_STORE.try_from_abi(type_info.into_iter(), type_table)
    }

    /// Returns the type that represents a pointer to this type
    pub fn pointer_type(&self, mutable: bool) -> Type {
        self.inner().pointer_type(mutable, &self.store)
    }

    /// Returns the type that represents an array to this type
    pub fn array_type(&self) -> Type {
        self.inner().array_type(&self.store)
    }

    /// Consumes the `Type`, returning a wrapped raw pointer.
    ///
    /// After calling this function, the caller is responsible for the memory
    /// previously managed by the `Type`. The easiest way to do this is to
    /// convert the raw pointer back into a `Type` with
    /// the [`Type::from_raw`] function, allowing the `Type` destructor to
    /// perform the cleanup.
    pub fn into_raw(ty: Type) -> *const std::ffi::c_void {
        ty.inner.as_ptr().cast()
    }

    /// Constructs a box from a raw pointer.
    ///
    /// After calling this function, the raw pointer is owned by the resulting
    /// `Type`. Specifically, the `Type` destructor will ensure the memory
    /// previously retained by the `raw` will be properly cleaned up. For
    /// this to be safe, the passed in `raw` pointer must have been
    /// previously returned by [`Type::into_raw`].
    ///
    /// This function must also not be called as part of static deinitialization
    /// as that may cause undefined behavior in the underlying
    /// implementation. Therefor passing the raw pointer over FFI might not
    /// be safe. Instead, wrap the `Type` in an `Arc` or a `Box` and use that on
    /// the FFI boundary.
    ///
    /// # Safety
    ///
    /// This function is unsafe because improper use may lead to memory
    /// problems. For example, a double-free may occur if the function is
    /// called twice on the same raw pointer.
    pub unsafe fn from_raw(raw: *const std::ffi::c_void) -> Type {
        Type {
            inner: NonNull::new(raw as *mut _).expect("invalid raw pointer"),
            store: GLOBAL_TYPE_STORE.clone(),
        }
    }
}

impl StructData {
    /// Tries to convert from an `abi::StructInfo`.
    fn try_from_abi<'abi>(
        struct_info: &'abi abi::StructDefinition<'abi>,
        type_table: &TypeTable,
    ) -> Result<StructData, TryFromAbiError<'abi>> {
        let fields: Result<Vec<FieldData>, TryFromAbiError<'abi>> = izip!(
            struct_info.field_names(),
            struct_info.field_types(),
            struct_info.field_offsets()
        )
        .map(|(name, type_id, offset)| {
            type_table
                .find_type_info_by_id(type_id)
                .ok_or_else(|| TryFromAbiError::UnknownTypeId(type_id.clone()))
                .map(|type_info| FieldData {
                    name: name.to_owned(),
                    type_info: type_info.inner,
                    offset: *offset,
                })
        })
        .collect();

        fields.map(|fields| StructData {
            guid: struct_info.guid,
            fields,
            memory_kind: struct_info.memory_kind,
        })
    }
}

/// A linked version of a struct field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldData {
    /// The field's name
    pub name: String,
    /// The field's type
    pub type_info: NonNull<TypeData>,
    /// The field's offset
    pub offset: u16,
    // TODO: Field accessibility levels
    // const MunPrivacy_t *field_privacies,
}

#[derive(Copy, Clone)]
pub struct Field<'t> {
    inner: &'t FieldData,
    store: &'t Arc<TypeDataStore>,
}

impl<'t> Field<'t> {
    /// Returns the name of the field
    pub fn name<'s>(&'s self) -> &'t str
    where
        't: 's,
    {
        self.inner.name.as_str()
    }

    /// Returns the type of the field
    pub fn ty(&self) -> Type {
        // Safety: this operation is safe due to the lifetime constraints on this type
        unsafe { Type::new_unchecked(self.inner.type_info, self.store.clone()) }
    }

    /// Returns the offset of the field from the start of the parent struct
    pub fn offset(&self) -> usize {
        self.inner.offset as _
    }
}

/// A helper struct to create a struct type.
pub struct StructTypeBuilder {
    /// The name of the struct type
    name: String,

    /// The type of memory management for this struct
    memory_kind: abi::StructMemoryKind,

    /// The fields of the struct
    fields: Vec<(String, Type, usize)>,

    /// Layout of the struct.
    layout: Layout,

    /// Optional explicit type of the struct
    guid: Option<abi::Guid>,
}

impl StructTypeBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            memory_kind: abi::StructMemoryKind::Gc,
            fields: Vec::new(),
            layout: Layout::from_size_align(0, 1).expect("invalid default layout"),
            guid: None,
        }
    }

    /// Sets the memory kind of the struct
    pub fn set_memory_kind(mut self, kind: abi::StructMemoryKind) -> Self {
        self.memory_kind = kind;
        self
    }

    /// Adds a field to the struct
    pub fn add_field(mut self, name: impl Into<String>, ty: Type) -> Self {
        let field_layout = if ty.is_value_type() {
            ty.value_layout()
        } else {
            Layout::new::<std::ffi::c_void>()
        };

        let (new_layout, offset) = self
            .layout
            .extend(field_layout)
            .expect("cannot extend struct layout");
        self.fields.push((name.into(), ty, offset));
        self.layout = new_layout;
        self
    }

    /// Adds a collection of fields to the struct
    pub fn add_fields<N: Into<String>>(
        mut self,
        iter: impl IntoIterator<Item = (N, Type)>,
    ) -> Self {
        for (name, ty) in iter {
            self = self.add_field(name.into(), ty);
        }
        self
    }

    /// Finishes building the struct returning the corresponding [`Type`].
    pub fn finish(self) -> Type {
        let guid = if let Some(guid) = self.guid {
            guid
        } else {
            let guid_string = build_struct_guid_string(
                &self.name,
                self.fields
                    .iter()
                    .map(|(name, ty, offset)| (name, Cow::Borrowed(ty), *offset)),
            );
            abi::Guid::from_str(&guid_string)
        };

        Type::new_struct(
            self.name,
            self.layout,
            guid,
            self.fields
                .into_iter()
                .map(|(name, ty, offset)| (name, ty, offset.try_into().expect("offset too large"))),
            self.memory_kind,
        )
    }
}

/// Constructs a string that unique identifies a struct with the given name and
/// fields.
fn build_struct_guid_string<'t, N: AsRef<str> + 't>(
    name: &str,
    fields: impl Iterator<Item = (N, Cow<'t, Type>, usize)>,
) -> String {
    let fields: Vec<String> = fields
        .map(|(name, ty, _offset)| {
            let ty_string = build_type_guid_string(ty.as_ref());
            format!("{}: {}", name.as_ref(), ty_string)
        })
        .collect();

    format!(
        "struct {name}{{{fields}}}",
        name = name,
        fields = fields.join(",")
    )
}

/// Constructs a string that unique identifies the specified type.
fn build_type_guid_string(ty: &Type) -> String {
    match ty.kind() {
        TypeKind::Struct(s) => {
            if s.is_gc_struct() {
                format!("struct {}", ty.name())
            } else {
                build_struct_guid_string(
                    ty.name(),
                    s.fields()
                        .iter()
                        .map(|f| (f.name(), Cow::Owned(f.ty()), f.offset())),
                )
            }
        }
        TypeKind::Array(_) | TypeKind::Primitive(_) | TypeKind::Pointer(_) => ty.name().to_owned(),
    }
}

/// A trait that defines static type information for types that can provide it.
pub trait HasStaticType {
    fn type_info() -> &'static Type;
}

macro_rules! impl_primitive_type {
    ($($ty:ty),+) => {
        $(
            impl HasStaticType for $ty {
                fn type_info() -> &'static Type {
                    static TYPE_INFO: once_cell::sync::OnceCell<Type> = once_cell::sync::OnceCell::new();
                    TYPE_INFO.get_or_init(|| {
                         GLOBAL_TYPE_STORE.allocate(
                             <$ty as abi::PrimitiveType>::name(),
                             Layout::new::<$ty>(),
                             TypeDataKind::Primitive(<$ty as abi::PrimitiveType>::guid().clone())
                         )
                    })
                }
            }
        )+
    }
}

impl_primitive_type!(
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    f32,
    f64,
    bool,
    (),
    std::ffi::c_void
);

/// Every type that has at least a type name also has a valid pointer type name
impl<T: HasStaticType + 'static> HasStaticType for *mut T {
    fn type_info() -> &'static Type {
        static mut VALUE: Option<StaticTypeMap<Type>> = None;
        static INIT: Once = Once::new();

        let map = unsafe {
            INIT.call_once(|| {
                VALUE = Some(StaticTypeMap::default());
            });
            VALUE.as_ref().unwrap()
        };

        map.call_once::<T, _>(|| T::type_info().pointer_type(true))
    }
}

/// Every type that has at least a type name also has a valid pointer type name
impl<T: HasStaticType + 'static> HasStaticType for *const T {
    fn type_info() -> &'static Type {
        static mut VALUE: Option<StaticTypeMap<Type>> = None;
        static INIT: Once = Once::new();

        let map = unsafe {
            INIT.call_once(|| {
                VALUE = Some(StaticTypeMap::default());
            });
            VALUE.as_ref().unwrap()
        };

        map.call_once::<T, _>(|| T::type_info().pointer_type(false))
    }
}

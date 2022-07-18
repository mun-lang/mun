mod ffi;

use std::{
    alloc::Layout,
    borrow::Cow,
    collections::VecDeque,
    fmt::{self, Formatter},
    fmt::{Debug, Display},
    hash::{Hash, Hasher},
    ops::Deref,
    ptr::NonNull,
    sync::{atomic::AtomicUsize, atomic::Ordering, Arc, Once},
};

use itertools::izip;
use once_cell::sync::Lazy;
use parking_lot::{lock_api::MutexGuard, Mutex, RawMutex, RwLock};

use abi::{self, static_type_map::StaticTypeMap};

use crate::{type_table::TypeTable, TryFromAbiError};

static GLOBAL_TYPE_STORE: Lazy<Arc<TypeStore>> = Lazy::new(Default::default);

#[no_mangle]
pub extern "C" fn hello_world() -> usize {
    3
}

#[derive(Default)]
struct TypeStore {
    types: Mutex<VecDeque<Box<TypeInner>>>,
}

impl TypeStore {
    /// Tries to convert multiple [`abi::TypeDefinition`] to internal type representations. If
    /// the conversion succeeds an updated [`TypeTable`] is returned
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
                TypeInnerData::Uninitialized,
                &mut entries,
            );
            type_table.insert_concrete_type(type_def.as_concrete().clone(), ty.clone());
            types.push(ty.clone());
            definition_and_type.push((type_def, ty));
        }

        // Next, initialize the types.
        for (type_def, mut ty) in definition_and_type {
            // Safety: we are modifying the inner data of the type here. At this point this is safe
            // because the type cannot be used by anything else yet.
            let inner_ty = unsafe { ty.inner.as_mut() };
            let type_data = match &type_def.data {
                abi::TypeDefinitionData::Struct(s) => {
                    StructInfo::try_from_abi(s, &type_table)?.into()
                }
            };
            inner_ty.data = type_data;
        }

        Ok((type_table, types))
    }

    fn allocate_inner(
        self: &Arc<Self>,
        name: impl Into<String>,
        layout: Layout,
        data: TypeInnerData,
        entries: &mut MutexGuard<'_, RawMutex, VecDeque<Box<TypeInner>>>,
    ) -> Type {
        entries.push_back(Box::new(TypeInner {
            name: name.into(),
            layout,
            data,
            external_references: AtomicUsize::new(0),
            immutable_pointer_type: Default::default(),
            mutable_pointer_type: Default::default(),
        }));

        // Safety: get a TypeInner with a 'static lifetime. This is safe because of the nature of
        // `Type`. The `Type` struct ensures that the pointed to value is never destructed as long
        // as it lives.
        let entry = unsafe {
            NonNull::new_unchecked(
                entries.back().expect("didnt insert").deref() as *const TypeInner as *mut _,
            )
        };

        // Safety: this operation is safe, because we currently own the instance and the lock.
        unsafe { Type::new_unchecked(entry, self.clone()) }
    }

    /// Allocates a new type instance
    pub fn allocate(
        self: &Arc<Self>,
        name: impl Into<String>,
        layout: Layout,
        data: TypeInnerData,
    ) -> Type {
        let mut entries = self.types.lock();
        self.allocate_inner(name, layout, data, &mut entries)
    }
}

/// A linked version of [`mun_abi::TypeInfo`] that has resolved all occurrences of `TypeId` with `TypeInfo`.
pub struct Type {
    inner: NonNull<TypeInner>,

    /// A [`Type`] holds a strong reference to its data store. This ensure that the data is never
    /// deleted before this instance is destroyed.
    store: Arc<TypeStore>,
}

impl Type {
    /// Constructs a new instance from a pointer and the store it belongs to.
    ///
    /// # Safety
    ///
    /// The pointer pointed to by `inner` might be invalid, in which case this method will cause
    /// undefined behavior.
    unsafe fn new_unchecked(mut inner: NonNull<TypeInner>, store: Arc<TypeStore>) -> Self {
        // Increment the external reference count
        inner
            .as_mut()
            .external_references
            .fetch_add(1, Ordering::Relaxed);

        Self { inner, store }
    }
}

unsafe impl Send for Type {}
unsafe impl Sync for Type {}

impl Debug for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        std::fmt::Display::fmt(&self.name(), f)
    }
}

impl Clone for Type {
    fn clone(&self) -> Self {
        // As explained in the [Boost documentation][1], Increasing the
        // reference counter can always be done with memory_order_relaxed: New
        // references to an object can only be formed from an existing
        // reference, and passing an existing reference from one thread to
        // another must already provide any required synchronization.
        self.inner()
            .external_references
            .fetch_add(1, Ordering::Relaxed);

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
        self.inner().hash(state)
    }
}

pub struct TypeInner {
    /// Type name
    name: String,

    /// The memory layout of the type
    layout: Layout,

    /// Type group
    data: TypeInnerData,

    /// Holds the number of external (non-cyclic) references
    external_references: AtomicUsize,

    /// The type of an immutable pointer to this type
    immutable_pointer_type: RwLock<Option<NonNull<TypeInner>>>,

    /// The type of a mutable pointer to this type
    mutable_pointer_type: RwLock<Option<NonNull<TypeInner>>>,
}

impl TypeInner {
    /// Returns the type that represents a pointer to this type
    fn pointer_type(&self, mutable: bool, store: &Arc<TypeStore>) -> Type {
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
                    inner: ty.clone(),
                    store: store.clone(),
                };
            }
        }

        let mut write_lock = cache_key.write();

        // Recheck if another thread acquired the write lock in the mean time
        if let Some(ty) = write_lock.deref() {
            return Type {
                inner: ty.clone(),
                store: store.clone(),
            };
        }

        // Otherwise create the type and store it
        let name = format!(
            "*{} {}",
            if mutable { "mut" } else { "const" },
            self.name
        );

        let ty = store.allocate(
            name,
            Layout::new::<*const std::ffi::c_void>(),
            PointerInfo {
                pointee: self.into(),
                mutable,
            }
            .into(),
        );
        *write_lock = Some(ty.inner);

        ty
    }
}

impl PartialEq for TypeInner {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.layout == other.layout && self.data == other.data
    }
}

impl Eq for TypeInner {}

unsafe impl Send for TypeInner {}
unsafe impl Sync for TypeInner {}

/// A linked version of [`mun_abi::TypeInfoData`] that has resolved all occurrences of `TypeId` with `TypeInfo`.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum TypeInnerData {
    /// Primitive types (i.e. `()`, `bool`, `float`, `int`, etc.)
    Primitive(abi::Guid),
    /// Struct types (i.e. record, tuple, or unit structs)
    Struct(StructInfo),
    /// A pointer to another type
    Pointer(PointerInfo),
    /// Indicates that the type has been allocated but it has not yet been initialized,
    /// this indicates that it still needs to be properly initialized.
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
}

/// A linked version of [`mun_abi::StructInfo`] that has resolved all occurrences of `TypeId` with `TypeInfo`.
#[derive(Clone, Debug)]
struct StructInfo {
    /// The unique identifier of this struct
    pub guid: abi::Guid,
    /// Struct fields
    pub fields: Vec<FieldInfo>,
    /// Struct memory kind
    pub memory_kind: abi::StructMemoryKind,
}

/// Reference information of a struct
#[repr(C)]
#[derive(Copy, Clone)]
pub struct StructType<'t> {
    inner: &'t StructInfo,
    store: &'t Arc<TypeStore>,
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

    /// Returns true if this struct is a value struct. Value structs are passed by value and are not
    /// allocated by the garbage collector.
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

/// A collection of fields of a struct
#[derive(Copy, Clone)]
pub struct Fields<'t> {
    inner: &'t StructInfo,
    store: &'t Arc<TypeStore>,
}

impl<'t> Fields<'t> {
    /// Returns the number of fields in the struct
    pub fn len(&self) -> usize {
        self.inner.fields.len()
    }

    /// Returns the field at the given index, or `None` if `index` exceeds the number of fields.
    pub fn get(&self, index: usize) -> Option<Field<'t>> {
        self.inner.fields.get(index).map(|field| Field {
            inner: field,
            store: self.store,
        })
    }

    /// Returns the field with the given name, or `None` if no such field exists.
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
    iter: std::slice::Iter<'t, FieldInfo>,
    store: &'t Arc<TypeStore>,
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

/// A linked version of [`mun_abi::PointerInfo`] that has resolved all occurrences of `TypeId` with `TypeInfo`.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct PointerInfo {
    /// The type to which is pointed
    pub pointee: NonNull<TypeInner>,
    /// Whether or not the pointer is mutable
    pub mutable: bool,
}

/// Reference information of a pointer
#[derive(Copy, Clone)]
pub struct PointerType<'t> {
    inner: &'t PointerInfo,
    store: &'t Arc<TypeStore>,
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

impl From<StructInfo> for TypeInnerData {
    fn from(s: StructInfo) -> Self {
        TypeInnerData::Struct(s)
    }
}

impl From<PointerInfo> for TypeInnerData {
    fn from(p: PointerInfo) -> Self {
        TypeInnerData::Pointer(p)
    }
}

impl Hash for TypeInner {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&self.data, state);
    }
}

impl Hash for StructInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.guid.hash(state)
    }
}

impl PartialEq for StructInfo {
    fn eq(&self, other: &Self) -> bool {
        self.guid == other.guid
    }
}
impl Eq for StructInfo {}

impl Type {
    /// Constructs a new struct type
    pub fn new_struct(
        name: impl Into<String>,
        layout: Layout,
        guid: abi::Guid,
        fields: impl IntoIterator<Item = (String, Type, u16)>,
        memory_kind: abi::StructMemoryKind,
    ) -> Type {
        GLOBAL_TYPE_STORE.allocate(
            name,
            layout,
            StructInfo {
                guid,
                fields: fields
                    .into_iter()
                    .map(|(name, ty, offset)| FieldInfo {
                        name,
                        type_info: ty.inner,
                        offset,
                    })
                    .collect(),
                memory_kind,
            }
            .into(),
        )
    }

    /// Returns a reference to the [`TypeInner`]
    fn inner(&self) -> &TypeInner {
        // Safety: taking the reference is always ok because the garbage collector ensures that as
        // long as self (Type) exists the inner stays alive.
        unsafe { self.inner.as_ref() }
    }

    /// Returns the name of the type
    pub fn name(&self) -> &str {
        self.inner().name.as_str()
    }

    /// Returns the type layout
    pub fn layout(&self) -> Layout {
        self.inner().layout
    }

    /// Returns true if this instance represents the TypeInfo of the given type.
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

    /// Returns the kind of the type
    pub fn kind(&self) -> TypeKind<'_> {
        match &self.inner().data {
            TypeInnerData::Primitive(guid) => TypeKind::Primitive(guid),
            TypeInnerData::Struct(s) => TypeKind::Struct(StructType {
                inner: s,
                store: &self.store,
            }),
            TypeInnerData::Pointer(p) => TypeKind::Pointer(PointerType {
                inner: p,
                store: &self.store,
            }),
            TypeInnerData::Uninitialized => {
                unreachable!("should never be able to query the kind of an uninitialized type")
            }
        }
    }

    /// Returns true if this type is a concrete type. This is the case for any type that doesn't
    /// refer to another type like a pointer.
    pub fn is_concrete(&self) -> bool {
        match self.kind() {
            TypeKind::Primitive(_) | TypeKind::Struct(_) => true,
            TypeKind::Pointer(_) => false,
        }
    }

    /// Returns the GUID associated with this instance if this instance represents a concrete type.
    pub fn as_concrete(&self) -> Option<&abi::Guid> {
        match self.kind() {
            TypeKind::Primitive(g) => Some(g),
            TypeKind::Struct(s) => Some(s.guid()),
            TypeKind::Pointer(_) => None,
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

    /// Returns whether the type is allocated on the stack.
    pub fn is_stack_allocated(&self) -> bool {
        match self.kind() {
            TypeKind::Primitive(_) | TypeKind::Pointer(_) => true,
            TypeKind::Struct(s) => s.is_value_struct(),
        }
    }

    /// Tries to convert multiple [`abi::TypeDefinition`] to internal type representations. If
    /// the conversion succeeds an updated [`TypeTable`] is returned
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

    /// Consumes the `Type`, returning a wrapped raw pointer.
    ///
    /// After calling this function, the caller is responsible for the memory previously managed by
    /// the `Type`. The easiest way to do this is to convert the raw pointer back into a `Type` with
    /// the [`Type::from_raw`] function, allowing the `Type` destructor to perform the cleanup.
    pub fn into_raw(ty: Type) -> *const std::ffi::c_void {
        ty.inner.as_ptr().cast()
    }

    /// Constructs a box from a raw pointer.
    ///
    /// After calling this function, the raw pointer is owned by the resulting `Type`. Specifically,
    /// the `Type` destructor will ensure the memory previously retained by the `raw` will be
    /// properly cleaned up. For this to be safe, the passed in `raw` pointer must have been
    /// previously returned by [`Type::into_raw`].
    ///
    /// This function must also not be called as part of static deinitialization as that may cause
    /// undefined behavior in the underlying implementation. Therefor passing the raw pointer over
    /// FFI might not be safe. Instead, wrap the `Type` in an `Arc` or a `Box` and use that on the
    /// FFI boundary.
    ///
    /// # Safety
    ///
    /// This function is unsafe because improper use may lead to memory problems. For example, a
    /// double-free may occur if the function is called twice on the same raw pointer.
    pub unsafe fn from_raw(raw: *const std::ffi::c_void) -> Type {
        Type {
            inner: NonNull::new(raw as *mut _).expect("invalid raw pointer"),
            store: GLOBAL_TYPE_STORE.clone(),
        }
    }
}

impl StructInfo {
    /// Tries to convert from an `abi::StructInfo`.
    fn try_from_abi<'abi>(
        struct_info: &'abi abi::StructDefinition<'abi>,
        type_table: &TypeTable,
    ) -> Result<StructInfo, TryFromAbiError<'abi>> {
        let fields: Result<Vec<FieldInfo>, TryFromAbiError> = izip!(
            struct_info.field_names(),
            struct_info.field_types(),
            struct_info.field_offsets()
        )
        .map(|(name, type_id, offset)| {
            type_table
                .find_type_info_by_id(type_id)
                .ok_or_else(|| TryFromAbiError::UnknownTypeId(type_id.clone()))
                .map(|type_info| FieldInfo {
                    name: name.to_owned(),
                    type_info: type_info.inner,
                    offset: *offset,
                })
        })
        .collect();

        fields.map(|fields| StructInfo {
            guid: struct_info.guid,
            fields,
            memory_kind: struct_info.memory_kind,
        })
    }
}

/// A linked version of a struct field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldInfo {
    /// The field's name
    pub name: String,
    /// The field's type
    pub type_info: NonNull<TypeInner>,
    /// The field's offset
    pub offset: u16,
    // TODO: Field accessibility levels
    // const MunPrivacy_t *field_privacies,
}

#[derive(Copy, Clone)]
pub struct Field<'t> {
    inner: &'t FieldInfo,
    store: &'t Arc<TypeStore>,
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
        let field_layout = if ty.is_stack_allocated() {
            ty.layout()
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
        for (name, ty) in iter.into_iter() {
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

        GLOBAL_TYPE_STORE.allocate(
            self.name,
            self.layout,
            StructInfo {
                guid,
                fields: Vec::from_iter(self.fields.into_iter().map(|(name, ty, offset)| {
                    FieldInfo {
                        name,
                        type_info: ty.inner,
                        offset: offset.try_into().expect("offset is too large!"),
                    }
                })),
                memory_kind: self.memory_kind,
            }
            .into(),
        )
    }
}

/// Constructs a string that unique identifies a struct with the given name and fields.
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
                    &ty.name(),
                    s.fields()
                        .iter()
                        .map(|f| (f.name(), Cow::Owned(f.ty()), f.offset())),
                )
            }
        }
        TypeKind::Primitive(_) | TypeKind::Pointer(_) => ty.name().to_owned(),
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
                             TypeInnerData::Primitive(<$ty as abi::PrimitiveType>::guid().clone())
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

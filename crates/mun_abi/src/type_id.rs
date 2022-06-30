use crate::static_type_map::StaticTypeMap;
use crate::Guid;
use once_cell::sync::OnceCell;
use std::fmt;

/// Represents a unique identifier for types. The runtime can use this to lookup the corresponding
/// [`TypeInfo`]. A [`TypeId`] is a key for a [`TypeInfo`].
///
/// A [`TypeId`] only contains enough information to query the runtime for a concrete type.
#[repr(u8)]
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum TypeId<'a> {
    /// Represents a concrete type with a specific Guid
    Concrete(Guid),

    /// Represents a pointer to a type
    Pointer(PointerTypeId<'a>),
}

/// Represents a pointer to another type.
#[repr(C)]
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PointerTypeId<'a> {
    /// The type to which this pointer points
    pub pointee: &'a TypeId<'a>,

    /// Whether or not this pointer is mutable or not
    pub mutable: bool,
}

unsafe impl<'a> Send for TypeId<'a> {}

unsafe impl<'a> Sync for TypeId<'a> {}

impl<'a> From<Guid> for TypeId<'a> {
    fn from(guid: Guid) -> Self {
        TypeId::Concrete(guid)
    }
}

impl<'a> From<PointerTypeId<'a>> for TypeId<'a> {
    fn from(pointer: PointerTypeId<'a>) -> Self {
        TypeId::Pointer(pointer)
    }
}

impl<'a> fmt::Display for TypeId<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeId::Concrete(guid) => guid.fmt(f),
            TypeId::Pointer(pointer) => pointer.fmt(f),
        }
    }
}

impl<'a> fmt::Display for PointerTypeId<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.mutable {
            write!(f, "*mut ")
        } else {
            write!(f, "*const ")
        }?;
        self.pointee.fmt(f)
    }
}

/// A trait that defines that for a type we can statically return a `TypeId`.
pub trait HasStaticTypeId {
    /// Returns a reference to the TypeInfo for the type
    fn type_id() -> &'static TypeId<'static>;
}

impl<T: HasStaticTypeId + 'static> HasStaticTypeId for *const T {
    fn type_id() -> &'static TypeId<'static> {
        static VALUE: OnceCell<StaticTypeMap<TypeId>> = OnceCell::new();
        let map = VALUE.get_or_init(Default::default);
        &map.call_once::<T, _>(|| {
            PointerTypeId {
                pointee: T::type_id(),
                mutable: false,
            }
            .into()
        })
    }
}

impl<T: HasStaticTypeId + 'static> HasStaticTypeId for *mut T {
    fn type_id() -> &'static TypeId<'static> {
        static VALUE: OnceCell<StaticTypeMap<TypeId>> = OnceCell::new();
        let map = VALUE.get_or_init(Default::default);
        &map.call_once::<T, _>(|| {
            PointerTypeId {
                pointee: T::type_id(),
                mutable: true,
            }
            .into()
        })
    }
}

use std::fmt;

use once_cell::sync::OnceCell;

use crate::{static_type_map::StaticTypeMap, Guid};

/// Represents a unique identifier for types. The runtime can use this to lookup
/// the corresponding [`TypeInfo`]. A [`TypeId`] is a key for a [`TypeInfo`].
///
/// A [`TypeId`] only contains enough information to query the runtime for a
/// [`TypeInfo`].
#[repr(u8)]
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum TypeId<'a> {
    /// Represents a concrete type with a specific Guid
    Concrete(Guid),

    /// Represents a pointer to a type
    Pointer(PointerTypeId<'a>),

    /// Represents an array of a specific type
    Array(ArrayTypeId<'a>),
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

/// Represents an array of a specific type.
#[repr(C)]
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ArrayTypeId<'a> {
    /// The element type of the array
    pub element: &'a TypeId<'a>,
}

unsafe impl Send for TypeId<'_> {}

unsafe impl Sync for TypeId<'_> {}

impl From<Guid> for TypeId<'_> {
    fn from(guid: Guid) -> Self {
        TypeId::Concrete(guid)
    }
}

impl<'a> From<PointerTypeId<'a>> for TypeId<'a> {
    fn from(pointer: PointerTypeId<'a>) -> Self {
        TypeId::Pointer(pointer)
    }
}

impl fmt::Display for TypeId<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeId::Concrete(guid) => guid.fmt(f),
            TypeId::Pointer(pointer) => pointer.fmt(f),
            TypeId::Array(array) => array.fmt(f),
        }
    }
}

impl fmt::Display for PointerTypeId<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.mutable {
            write!(f, "*mut ")
        } else {
            write!(f, "*const ")
        }?;
        self.pointee.fmt(f)
    }
}

impl fmt::Display for ArrayTypeId<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", &self.element)
    }
}

/// A trait that defines that for a type we can statically return a `TypeId`.
pub trait HasStaticTypeId {
    /// Returns a reference to the [`TypeInfo`] for the type
    fn type_id() -> &'static TypeId<'static>;
}

impl<T: HasStaticTypeId + 'static> HasStaticTypeId for *const T {
    fn type_id() -> &'static TypeId<'static> {
        static VALUE: OnceCell<StaticTypeMap<TypeId<'static>>> = OnceCell::new();
        let map = VALUE.get_or_init(Default::default);
        map.call_once::<T, _>(|| {
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
        static VALUE: OnceCell<StaticTypeMap<TypeId<'static>>> = OnceCell::new();
        let map = VALUE.get_or_init(Default::default);
        map.call_once::<T, _>(|| {
            PointerTypeId {
                pointee: T::type_id(),
                mutable: true,
            }
            .into()
        })
    }
}

#[cfg(test)]
mod test {
    use crate::{ArrayTypeId, HasStaticTypeId, PointerTypeId, PrimitiveType, TypeId};

    #[test]
    fn display() {
        assert_eq!(i32::type_id().to_string(), i32::guid().to_string());
        assert_eq!(f64::type_id().to_string(), f64::guid().to_string());
        assert_eq!(
            std::ffi::c_void::type_id().to_string(),
            std::ffi::c_void::guid().to_string()
        );

        let i32_type_id = i32::type_id();
        assert_eq!(
            TypeId::Pointer(PointerTypeId {
                pointee: i32_type_id,
                mutable: false
            })
            .to_string(),
            format!("*const {}", i32::guid())
        );
        assert_eq!(
            TypeId::Pointer(PointerTypeId {
                pointee: i32_type_id,
                mutable: true
            })
            .to_string(),
            format!("*mut {}", i32::guid())
        );

        assert_eq!(
            TypeId::Array(ArrayTypeId {
                element: i32_type_id
            })
            .to_string(),
            format!("[{}]", i32::guid())
        );
    }
}

//! A module that defines information for built-in (or primitive) types.

use crate::{Guid, HasStaticTypeId, TypeId};

/// Defines functions for built-in types like f32, i32, etc.
pub trait PrimitiveType: HasStaticTypeId {
    /// Returns the name of the type
    fn name() -> &'static str;

    /// Returns the GUID of the type
    fn guid() -> &'static Guid;
}

macro_rules! define_primitives {
    ($($ty:ty => $name:literal),*) => {
        $(
            impl HasStaticTypeId for $ty {
                fn type_id() -> &'static $crate::TypeId<'static> {
                    const TYPE_ID: $crate::TypeId<'static> = $crate::TypeId::Concrete(Guid::from_str($name));
                    &TYPE_ID
                }
            }

            impl PrimitiveType for $ty {
                fn name() -> &'static str {
                    const TYPE_NAME: &str = $name;
                    TYPE_NAME
                }

                fn guid() -> &'static Guid {
                    const TYPE_GUID: Guid = Guid::from_str($name);
                    &TYPE_GUID
                }
            }
        )+
    }
}

define_primitives! {
    i8 => "core::i8",
    i16 => "core::i16",
    i32 => "core::i32",
    i64 => "core::i64",
    i128 => "core::i128",
    u8 => "core::u8",
    u16 => "core::u16",
    u32 => "core::u32",
    u64 => "core::u64",
    u128 => "core::u128",
    f32 => "core::f32",
    f64 => "core::f64",
    bool => "core::bool",
    () => "core::empty",
    std::ffi::c_void => "core::void"
}

#[cfg(target_pointer_width = "64")]
impl PrimitiveType for usize {
    fn name() -> &'static str {
        u64::name()
    }
    fn guid() -> &'static Guid {
        u64::guid()
    }
}

#[cfg(target_pointer_width = "64")]
impl HasStaticTypeId for usize {
    fn type_id() -> &'static TypeId<'static> {
        u64::type_id()
    }
}

#[cfg(target_pointer_width = "64")]
impl PrimitiveType for isize {
    fn name() -> &'static str {
        i64::name()
    }
    fn guid() -> &'static Guid {
        i64::guid()
    }
}

#[cfg(target_pointer_width = "64")]
impl HasStaticTypeId for isize {
    fn type_id() -> &'static TypeId<'static> {
        i64::type_id()
    }
}

#[cfg(target_pointer_width = "32")]
impl PrimitiveType for usize {
    fn name() -> &'static str {
        u32::name()
    }
    fn guid() -> &'static Guid {
        u32::guid()
    }
}

#[cfg(target_pointer_width = "32")]
impl HasStaticTypeId for usize {
    fn type_id() -> &'static TypeId<'static> {
        u32::type_id()
    }
}

#[cfg(target_pointer_width = "32")]
impl PrimitiveType for isize {
    fn name() -> &'static str {
        i32::name()
    }
    fn guid() -> &'static Guid {
        i32::guid()
    }
}

#[cfg(target_pointer_width = "32")]
impl HasStaticTypeId for isize {
    fn type_id() -> &'static TypeId<'static> {
        i32::type_id()
    }
}

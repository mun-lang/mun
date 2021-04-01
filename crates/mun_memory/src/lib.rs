use std::alloc::Layout;

mod cast;
pub mod diff;
pub mod gc;
pub mod mapping;

pub mod prelude {
    pub use crate::diff::{diff, Diff, FieldDiff, FieldEditKind};
    pub use crate::mapping::{Action, FieldMapping};
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum TypeGroup {
    Primitive,
    Struct,
}

impl<'t> From<&'t abi::TypeInfoData> for TypeGroup {
    fn from(data: &'t abi::TypeInfoData) -> Self {
        match data {
            abi::TypeInfoData::Primitive => TypeGroup::Primitive,
            abi::TypeInfoData::Struct(_) => TypeGroup::Struct,
        }
    }
}

/// A trait used to obtain a type's description.
pub trait TypeDesc: Send + Sync {
    /// Returns the name of this type.
    fn name(&self) -> &str;
    /// Returns the `Guid` of this type.
    fn guid(&self) -> &abi::Guid;
    /// Returns the `TypeGroup` of this type.
    fn group(&self) -> TypeGroup;
}

/// A trait used to obtain a type's memory description.
pub trait TypeMemory: Send + Sync {
    /// Returns the memory layout of this type.
    fn layout(&self) -> Layout;
    /// Returns whether the memory is stack-allocated.
    fn is_stack_allocated(&self) -> bool;
}

/// A trait used to obtain a type's fields.
pub trait TypeFields<T>: Send + Sync {
    /// Returns the type's fields.
    fn fields(&self) -> Vec<(&str, T)>;
    /// Returns the type's fields' offsets.
    fn offsets(&self) -> &[u16];
}

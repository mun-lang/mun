use std::alloc::Layout;

mod diff;
pub mod mapping;
pub mod myers;

pub use {
    diff::{diff, Diff, FieldDiff, FieldEditKind},
    mapping::{Action, FieldMappingDesc},
};

/// A trait used to obtain a type's description.
pub trait TypeDesc: Send + Sync {
    /// Returns the name of this type.
    fn name(&self) -> &str;
    /// Returns the `Guid` of this type.
    fn guid(&self) -> &abi::Guid;
    /// Returns the `TypeGroup` of this type.
    fn group(&self) -> abi::TypeGroup;
}

/// A trait used to obtain a type's memory layout.
pub trait TypeLayout: Send + Sync {
    /// Returns the memory layout of this type.
    fn layout(&self) -> Layout;
}

/// A trait used to obtain a type's fields.
pub trait TypeFields<T>: Send + Sync {
    /// Returns the type's fields.
    fn fields(&self) -> Vec<(&str, T)>;
    /// Returns the type's fields' offsets.
    fn offsets(&self) -> &[u16];
}

pub trait MemoryMapper<T> {
    /// Maps the values memory from `old` to `new` using `diff`.
    fn map_memory(&self, old: &[T], new: &[T], diff: &[Diff]);
}

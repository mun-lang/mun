use std::alloc::Layout;

pub mod myers;

/// A trait used to obtain a type's memory layout.
pub trait TypeLayout: Send + Sync {
    /// Returns the memory layout of this type.
    fn layout(&self) -> Layout;
}

/// A trait used to obtain a type's description.
pub trait TypeDesc: Send + Sync {
    /// Returns the name of this type.
    fn name(&self) -> &str;
    /// Returns the `Guid` of this type.
    fn guid(&self) -> &abi::Guid;
    /// Returns the `TypeGroup` of this type.
    fn group(&self) -> abi::TypeGroup;
}

#[derive(Debug)]
pub enum Diff<'t, T> {
    Insert { value: &'t T, index: usize },
    Delete { index: usize },
}

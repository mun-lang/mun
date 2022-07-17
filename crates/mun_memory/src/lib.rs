extern crate core;

pub use r#type::{FieldInfo, HasStaticType, StructType, PointerType, TypeKind, Type, Field, StructTypeBuilder};

mod cast;
pub mod diff;
pub mod gc;
pub mod mapping;
mod r#type;
pub mod type_table;
use thiserror::Error;

pub mod prelude {
    pub use crate::diff::{diff, Diff, FieldDiff, FieldEditKind};
    pub use crate::mapping::{Action, FieldMapping};
    pub use crate::r#type::{StructType, PointerType, Type, TypeKind, Field};
}

/// An error that can occur when trying to convert from an abi type to an internal type.
#[derive(Debug, Error)]
pub enum TryFromAbiError<'a> {
    #[error("unknown TypeId '{0}'")]
    UnknownTypeId(abi::TypeId<'a>),
}

extern crate core;

pub use r#type::{
    ArrayType, Field, FieldData, HasStaticType, PointerType, StructType, StructTypeBuilder, Type,
    TypeCollectionStats, TypeKind,
};

pub mod ffi {
    pub use super::r#type::ffi::*;
}

mod cast;
pub mod diff;
pub mod gc;
pub mod mapping;
mod r#type;
pub mod type_table;
use mun_abi as abi;

pub mod prelude {
    pub use crate::diff::{compute_struct_diff, FieldDiff, FieldEditKind, StructDiff};
    pub use crate::mapping::{Action, FieldMapping};
    pub use crate::r#type::{Field, PointerType, StructType, Type, TypeKind};
}

/// An error that can occur when trying to convert from an abi type to an internal type.
#[derive(Debug, thiserror::Error)]
pub enum TryFromAbiError<'a> {
    #[error("unknown TypeId '{0}'")]
    UnknownTypeId(abi::TypeId<'a>),
}

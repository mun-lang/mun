use memory::TypeInfo;

use crate::Runtime;
use std::{ptr::NonNull, sync::Arc};

/// Used to do value-to-value conversions that require runtime type information while consuming the
/// input value.
///
/// If no `TypeInfo` is provided, the type is `()`.
pub trait Marshal<'t>: Sized {
    /// The type used in the Mun ABI
    type MunType;

    /// Marshals from a value (i.e. Mun -> Rust).
    fn marshal_from<'r>(value: Self::MunType, runtime: &'r Runtime) -> Self
    where
        Self: 't,
        'r: 't;

    /// Marshals itself into a `Marshalled` value (i.e. Rust -> Mun).
    fn marshal_into(self) -> Self::MunType;

    /// Marshals the value at memory location `ptr` into a `Marshalled` value (i.e. Mun -> Rust).
    fn marshal_from_ptr<'r>(
        ptr: NonNull<Self::MunType>,
        runtime: &'r Runtime,
        type_info: &Arc<TypeInfo>,
    ) -> Self
    where
        Self: 't,
        'r: 't;

    /// Marshals `value` to memory location `ptr` (i.e. Rust -> Mun).
    fn marshal_to_ptr(
        value: Self,
        ptr: NonNull<Self::MunType>,
        runtime: &Runtime,
        type_info: &Arc<TypeInfo>,
    );
}

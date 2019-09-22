use crate::prelude::*;
use md5;

/// A type to emulate dynamic typing across compilation units for static types.
pub trait Reflection: 'static {
    /// Retrieves the type's `Guid`.
    fn type_guid() -> Guid {
        Guid {
            b: md5::compute(Self::type_name()).0,
        }
    }

    /// Retrieves the type's name.
    fn type_name() -> &'static str;
}

impl Reflection for f64 {
    fn type_name() -> &'static str {
        "@core::float"
    }
}

impl Reflection for i64 {
    fn type_name() -> &'static str {
        "@core::int"
    }
}

impl Reflection for bool {
    fn type_name() -> &'static str {
        "@core::bool"
    }
}

impl Reflection for () {
    fn type_name() -> &'static str {
        "@core::empty"
    }
}

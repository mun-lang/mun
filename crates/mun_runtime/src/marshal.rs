use abi::TypeInfo;

/// Used to do value-to-value conversions that require runtime type information while consuming the
/// input value.
///
/// If no `TypeInfo` is provided, the type is `()`.
pub trait MarshalInto<T>: Sized {
    /// Performs the conversion.
    fn marshal_into(self, type_info: Option<&TypeInfo>) -> T;
}

impl<T> MarshalInto<T> for T {
    fn marshal_into(self, _type_info: Option<&TypeInfo>) -> T {
        self
    }
}

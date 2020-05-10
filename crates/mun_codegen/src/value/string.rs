use super::{AsValue, Global, IrValueContext, TransparentValue, Value};
use std::ffi::{CStr, CString};

/// Enables internalizing certain data structures like strings.
pub trait CanInternalize {
    type Type;

    /// Internalizes the instance into a global value.
    fn intern<S: AsRef<str>>(&self, name: S, context: &IrValueContext) -> Global<Self::Type>;
}

impl CanInternalize for str {
    type Type = String;

    fn intern<S: AsRef<str>>(&self, name: S, context: &IrValueContext) -> Global<Self::Type> {
        Global::from_raw(
            self.as_bytes()
                .as_value(context)
                .into_const_private_global(name, context)
                .value,
        )
    }
}

impl CanInternalize for CStr {
    type Type = CString;

    fn intern<S: AsRef<str>>(&self, name: S, context: &IrValueContext) -> Global<Self::Type> {
        Global::from_raw(
            self.to_bytes_with_nul()
                .as_value(context)
                .into_const_private_global(name, context)
                .value,
        )
    }
}

impl TransparentValue for CString {
    type Target = [u8];

    fn as_target_value(&self, context: &IrValueContext) -> Value<Self::Target> {
        self.as_bytes_with_nul().as_value(context)
    }
}

impl TransparentValue for String {
    type Target = [u8];

    fn as_target_value(&self, context: &IrValueContext) -> Value<Self::Target> {
        self.as_bytes().as_value(context)
    }
}

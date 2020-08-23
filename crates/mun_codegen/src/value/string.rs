use super::{AsValue, Global, IrValueContext, TransparentValue, Value};
use std::ffi::{CStr, CString};

/// Enables internalizing certain data structures like strings.
pub trait CanInternalize {
    type Type;

    /// Internalizes the instance into a global value.
    fn intern<'ink, S: AsRef<str>>(
        &self,
        name: S,
        context: &IrValueContext<'ink, '_, '_>,
    ) -> Global<'ink, Self::Type>;
}

impl CanInternalize for str {
    type Type = String;

    fn intern<'ink, S: AsRef<str>>(
        &self,
        name: S,
        context: &IrValueContext<'ink, '_, '_>,
    ) -> Global<'ink, Self::Type> {
        unsafe {
            Global::from_raw(
                self.as_bytes()
                    .as_value(context)
                    .into_const_private_global(name, context)
                    .value,
            )
        }
    }
}

impl CanInternalize for CStr {
    type Type = CString;

    fn intern<'ink, S: AsRef<str>>(
        &self,
        name: S,
        context: &IrValueContext<'ink, '_, '_>,
    ) -> Global<'ink, Self::Type> {
        unsafe {
            Global::from_raw(
                self.to_bytes_with_nul()
                    .as_value(context)
                    .into_const_private_global(name, context)
                    .value,
            )
        }
    }
}

impl<'ink> TransparentValue<'ink> for CString {
    type Target = [u8];

    fn as_target_value(&self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, Self::Target> {
        self.as_bytes_with_nul().as_value(context)
    }
}

impl<'ink> TransparentValue<'ink> for String {
    type Target = [u8];

    fn as_target_value(&self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, Self::Target> {
        self.as_bytes().as_value(context)
    }
}

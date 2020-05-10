use crate::value::{AsValue, Global, IrValueContext, SizedValueType, TransparentValue, Value};
use std::ffi::CString;

impl TransparentValue for abi::Guid {
    type Target = [u8; 16];

    fn as_target_value(&self, context: &IrValueContext) -> Value<Self::Target> {
        self.b.as_value(context)
    }
}

impl TransparentValue for abi::Privacy {
    type Target = u8;

    fn as_target_value(&self, context: &IrValueContext) -> Value<Self::Target> {
        (*self as u8).as_value(context)
    }
}

impl TransparentValue for abi::TypeGroup {
    type Target = u8;

    fn as_target_value(&self, context: &IrValueContext) -> Value<Self::Target> {
        (*self as u8).as_value(context)
    }
}

#[derive(AsValue)]
#[ir_name = "struct.MunTypeInfo"]
pub struct TypeInfo {
    guid: abi::Guid,
    name: Value<*const Global<CString>>,
    size_in_bits: u32,
    alignment: u8,
    type_group: abi::TypeGroup,
}

#[derive(AsValue)]
#[ir_name = "struct.MunFunctionSignature"]
pub struct FunctionSignature {
    arg_types: Value<*const *const TypeInfo>,
    return_type: Value<*const TypeInfo>,
    num_arg_types: u16,
}

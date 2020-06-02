use super::{
    AddressableType, AsValue, ConcreteValueType, IrTypeContext, IrValueContext, PointerValueType,
    SizedValueType, Value,
};
use inkwell::AddressSpace;

macro_rules! impl_as_int_ir_value {
    ($($ty:ty => $context_fun:ident()),*) => {
        $(
            impl ConcreteValueType for $ty {
                type Value = inkwell::values::IntValue;
            }

            impl SizedValueType for $ty {
                fn get_ir_type(context: &IrTypeContext) -> inkwell::types::IntType {
                    context.context.$context_fun()
                }
            }

            impl PointerValueType for $ty {
                fn get_ptr_type(context: &IrTypeContext, address_space: Option<AddressSpace>) -> inkwell::types::PointerType {
                    Self::get_ir_type(context).ptr_type(address_space.unwrap_or(AddressSpace::Generic))
                }
            }

            impl AddressableType<$ty> for $ty {}
        )*
    }
}

impl_as_int_ir_value!(
    i8 => i8_type(),
    i16 => i16_type(),
    i32 => i32_type(),
    i64 => i64_type(),
    u8 => i8_type(),
    u16 => i16_type(),
    u32 => i32_type(),
    u64 => i64_type()
);

impl AsValue<u8> for u8 {
    fn as_value(&self, context: &IrValueContext) -> Value<u8> {
        Value::from_raw(
            <Self as SizedValueType>::get_ir_type(context.type_context)
                .const_int(*self as u64, false),
        )
    }
}

impl AsValue<u16> for u16 {
    fn as_value(&self, context: &IrValueContext) -> Value<u16> {
        Value::from_raw(
            <Self as SizedValueType>::get_ir_type(context.type_context)
                .const_int(*self as u64, false),
        )
    }
}

impl AsValue<u32> for u32 {
    fn as_value(&self, context: &IrValueContext) -> Value<u32> {
        Value::from_raw(
            <Self as SizedValueType>::get_ir_type(context.type_context)
                .const_int(*self as u64, false),
        )
    }
}

impl AsValue<u64> for u64 {
    fn as_value(&self, context: &IrValueContext) -> Value<u64> {
        Value::from_raw(
            <Self as SizedValueType>::get_ir_type(context.type_context)
                .const_int(*self as u64, false),
        )
    }
}

impl AsValue<i8> for i8 {
    fn as_value(&self, context: &IrValueContext) -> Value<i8> {
        Value::from_raw(
            <Self as SizedValueType>::get_ir_type(context.type_context)
                .const_int(*self as u64, true),
        )
    }
}

impl AsValue<i16> for i16 {
    fn as_value(&self, context: &IrValueContext) -> Value<i16> {
        Value::from_raw(
            <Self as SizedValueType>::get_ir_type(context.type_context)
                .const_int(*self as u64, true),
        )
    }
}

impl AsValue<i32> for i32 {
    fn as_value(&self, context: &IrValueContext) -> Value<i32> {
        Value::from_raw(
            <Self as SizedValueType>::get_ir_type(context.type_context)
                .const_int(*self as u64, true),
        )
    }
}

impl AsValue<i64> for i64 {
    fn as_value(&self, context: &IrValueContext) -> Value<i64> {
        Value::from_raw(
            <Self as SizedValueType>::get_ir_type(context.type_context)
                .const_int(*self as u64, true),
        )
    }
}

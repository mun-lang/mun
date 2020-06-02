use super::{
    AsValue, ConcreteValueType, IrTypeContext, IrValueContext, PointerValueType, SizedValueType,
    Value,
};
use crate::value::AddressableType;
use inkwell::{types::PointerType, AddressSpace};

impl ConcreteValueType for f32 {
    type Value = inkwell::values::FloatValue;
}

impl ConcreteValueType for f64 {
    type Value = inkwell::values::FloatValue;
}

impl SizedValueType for f32 {
    fn get_ir_type(context: &IrTypeContext) -> inkwell::types::FloatType {
        context.context.f32_type()
    }
}
impl SizedValueType for f64 {
    fn get_ir_type(context: &IrTypeContext) -> inkwell::types::FloatType {
        context.context.f64_type()
    }
}

impl PointerValueType for f32 {
    fn get_ptr_type(context: &IrTypeContext, address_space: Option<AddressSpace>) -> PointerType {
        Self::get_ir_type(context).ptr_type(address_space.unwrap_or(AddressSpace::Generic))
    }
}
impl PointerValueType for f64 {
    fn get_ptr_type(context: &IrTypeContext, address_space: Option<AddressSpace>) -> PointerType {
        Self::get_ir_type(context).ptr_type(address_space.unwrap_or(AddressSpace::Generic))
    }
}

impl AddressableType<f32> for f32 {}
impl AddressableType<f64> for f64 {}

impl AsValue<f32> for f32 {
    fn as_value(&self, context: &IrValueContext) -> Value<f32> {
        Value::from_raw(
            <Self as SizedValueType>::get_ir_type(context.type_context).const_float(*self as f64),
        )
    }
}
impl AsValue<f64> for f64 {
    fn as_value(&self, context: &IrValueContext) -> Value<f64> {
        Value::from_raw(
            <Self as SizedValueType>::get_ir_type(context.type_context).const_float(*self),
        )
    }
}

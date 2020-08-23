use super::{
    AsValue, ConcreteValueType, IrTypeContext, IrValueContext, PointerValueType, SizedValueType,
    Value,
};
use crate::value::AddressableType;
use inkwell::{types::PointerType, AddressSpace};

impl<'ink> ConcreteValueType<'ink> for f32 {
    type Value = inkwell::values::FloatValue<'ink>;
}

impl<'ink> ConcreteValueType<'ink> for f64 {
    type Value = inkwell::values::FloatValue<'ink>;
}

impl<'ink> SizedValueType<'ink> for f32 {
    fn get_ir_type(context: &IrTypeContext<'ink, '_>) -> inkwell::types::FloatType<'ink> {
        context.context.f32_type()
    }
}
impl<'ink> SizedValueType<'ink> for f64 {
    fn get_ir_type(context: &IrTypeContext<'ink, '_>) -> inkwell::types::FloatType<'ink> {
        context.context.f64_type()
    }
}

impl<'ink> PointerValueType<'ink> for f32 {
    fn get_ptr_type(
        context: &IrTypeContext<'ink, '_>,
        address_space: Option<AddressSpace>,
    ) -> PointerType<'ink> {
        Self::get_ir_type(context).ptr_type(address_space.unwrap_or(AddressSpace::Generic))
    }
}
impl<'ink> PointerValueType<'ink> for f64 {
    fn get_ptr_type(
        context: &IrTypeContext<'ink, '_>,
        address_space: Option<AddressSpace>,
    ) -> PointerType<'ink> {
        Self::get_ir_type(context).ptr_type(address_space.unwrap_or(AddressSpace::Generic))
    }
}

impl<'ink> AddressableType<'ink, f32> for f32 {}
impl<'ink> AddressableType<'ink, f64> for f64 {}

impl<'ink> AsValue<'ink, f32> for f32 {
    fn as_value(&self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, f32> {
        Value::from_raw(
            <Self as SizedValueType>::get_ir_type(context.type_context).const_float(*self as f64),
        )
    }
}
impl<'ink> AsValue<'ink, f64> for f64 {
    fn as_value(&self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, f64> {
        Value::from_raw(
            <Self as SizedValueType>::get_ir_type(context.type_context).const_float(*self),
        )
    }
}

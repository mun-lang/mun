use super::{
    AddressableType, AsBytesAndPtrs, AsValue, BytesOrPtr, ConcreteValueType, HasConstValue,
    IrTypeContext, IrValueContext, PointerValueType, SizedValueType, Value,
};
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
        Self::get_ir_type(context).ptr_type(address_space.unwrap_or(AddressSpace::default()))
    }
}
impl<'ink> PointerValueType<'ink> for f64 {
    fn get_ptr_type(
        context: &IrTypeContext<'ink, '_>,
        address_space: Option<AddressSpace>,
    ) -> PointerType<'ink> {
        Self::get_ir_type(context).ptr_type(address_space.unwrap_or(AddressSpace::default()))
    }
}

impl<'ink> AddressableType<'ink, f32> for f32 {}
impl<'ink> AddressableType<'ink, f64> for f64 {}

impl HasConstValue for f32 {
    fn has_const_value() -> bool {
        true
    }
}

impl HasConstValue for f64 {
    fn has_const_value() -> bool {
        true
    }
}

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

impl<'ink> AsBytesAndPtrs<'ink> for f32 {
    fn as_bytes_and_ptrs(&self, _: &IrTypeContext<'ink, '_>) -> Vec<BytesOrPtr<'ink>> {
        vec![bytemuck::cast_ref::<f32, [u8; 4]>(self).to_vec().into()]
    }
}

impl<'ink> AsBytesAndPtrs<'ink> for f64 {
    fn as_bytes_and_ptrs(&self, _: &IrTypeContext<'ink, '_>) -> Vec<BytesOrPtr<'ink>> {
        vec![bytemuck::cast_ref::<f64, [u8; 8]>(self).to_vec().into()]
    }
}

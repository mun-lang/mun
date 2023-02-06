use super::{
    AddressableType, AsBytesAndPtrs, AsValue, BytesOrPtr, ConcreteValueType, HasConstValue,
    IrTypeContext, IrValueContext, PointerValueType, SizedValueType, Value,
};
use inkwell::AddressSpace;

macro_rules! impl_as_int_ir_value {
    ($($ty:ty => $context_fun:ident()),*) => {
        $(
            impl<'ink> ConcreteValueType<'ink> for $ty {
                type Value = inkwell::values::IntValue<'ink>;
            }

            impl<'ink> SizedValueType<'ink> for $ty {
                fn get_ir_type(context: &IrTypeContext<'ink, '_>) -> inkwell::types::IntType<'ink> {
                    context.context.$context_fun()
                }
            }

            impl<'ink> PointerValueType<'ink> for $ty {
                fn get_ptr_type(context: &IrTypeContext<'ink, '_>, address_space: Option<AddressSpace>) -> inkwell::types::PointerType<'ink> {
                    Self::get_ir_type(context).ptr_type(address_space.unwrap_or(AddressSpace::Generic))
                }
            }

            impl<'ink> AddressableType<'ink, $ty> for $ty {}

            impl HasConstValue for $ty {
                fn has_const_value() -> bool {
                    true
                }
            }
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
    u64 => i64_type(),
    bool => bool_type()
);

impl<'ink> AsValue<'ink, u8> for u8 {
    fn as_value(&self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, u8> {
        Value::from_raw(
            <Self as SizedValueType>::get_ir_type(context.type_context)
                .const_int(*self as u64, false),
        )
    }
}

impl<'ink> AsValue<'ink, u16> for u16 {
    fn as_value(&self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, u16> {
        Value::from_raw(
            <Self as SizedValueType>::get_ir_type(context.type_context)
                .const_int(*self as u64, false),
        )
    }
}

impl<'ink> AsValue<'ink, u32> for u32 {
    fn as_value(&self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, u32> {
        Value::from_raw(
            <Self as SizedValueType>::get_ir_type(context.type_context)
                .const_int(*self as u64, false),
        )
    }
}

impl<'ink> AsValue<'ink, u64> for u64 {
    fn as_value(&self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, u64> {
        Value::from_raw(
            <Self as SizedValueType>::get_ir_type(context.type_context).const_int(*self, false),
        )
    }
}

impl<'ink> AsValue<'ink, i8> for i8 {
    fn as_value(&self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, i8> {
        Value::from_raw(
            <Self as SizedValueType>::get_ir_type(context.type_context)
                .const_int(*self as u64, true),
        )
    }
}

impl<'ink> AsValue<'ink, bool> for bool {
    fn as_value(&self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, bool> {
        Value::from_raw(
            <Self as SizedValueType>::get_ir_type(context.type_context)
                .const_int(*self as u64, true),
        )
    }
}

impl<'ink> AsValue<'ink, i16> for i16 {
    fn as_value(&self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, i16> {
        Value::from_raw(
            <Self as SizedValueType>::get_ir_type(context.type_context)
                .const_int(*self as u64, true),
        )
    }
}

impl<'ink> AsValue<'ink, i32> for i32 {
    fn as_value(&self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, i32> {
        Value::from_raw(
            <Self as SizedValueType>::get_ir_type(context.type_context)
                .const_int(*self as u64, true),
        )
    }
}

impl<'ink> AsValue<'ink, i64> for i64 {
    fn as_value(&self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, i64> {
        Value::from_raw(
            <Self as SizedValueType>::get_ir_type(context.type_context)
                .const_int(*self as u64, true),
        )
    }
}

impl<'ink> AsBytesAndPtrs<'ink> for u8 {
    fn as_bytes_and_ptrs(&self, _: &IrTypeContext<'ink, '_>) -> Vec<BytesOrPtr<'ink>> {
        vec![bytemuck::cast_ref::<u8, [u8; 1]>(self).to_vec().into()]
    }
}

impl<'ink> AsBytesAndPtrs<'ink> for u16 {
    fn as_bytes_and_ptrs(&self, _: &IrTypeContext<'ink, '_>) -> Vec<BytesOrPtr<'ink>> {
        vec![bytemuck::cast_ref::<u16, [u8; 2]>(self).to_vec().into()]
    }
}

impl<'ink> AsBytesAndPtrs<'ink> for u32 {
    fn as_bytes_and_ptrs(&self, _: &IrTypeContext<'ink, '_>) -> Vec<BytesOrPtr<'ink>> {
        vec![bytemuck::cast_ref::<u32, [u8; 4]>(self).to_vec().into()]
    }
}

impl<'ink> AsBytesAndPtrs<'ink> for u64 {
    fn as_bytes_and_ptrs(&self, _: &IrTypeContext<'ink, '_>) -> Vec<BytesOrPtr<'ink>> {
        vec![bytemuck::cast_ref::<u64, [u8; 8]>(self).to_vec().into()]
    }
}

impl<'ink> AsBytesAndPtrs<'ink> for i8 {
    fn as_bytes_and_ptrs(&self, _: &IrTypeContext<'ink, '_>) -> Vec<BytesOrPtr<'ink>> {
        vec![bytemuck::cast_ref::<i8, [u8; 1]>(self).to_vec().into()]
    }
}

impl<'ink> AsBytesAndPtrs<'ink> for i16 {
    fn as_bytes_and_ptrs(&self, _: &IrTypeContext<'ink, '_>) -> Vec<BytesOrPtr<'ink>> {
        vec![bytemuck::cast_ref::<i16, [u8; 2]>(self).to_vec().into()]
    }
}

impl<'ink> AsBytesAndPtrs<'ink> for i32 {
    fn as_bytes_and_ptrs(&self, _: &IrTypeContext<'ink, '_>) -> Vec<BytesOrPtr<'ink>> {
        vec![bytemuck::cast_ref::<i32, [u8; 4]>(self).to_vec().into()]
    }
}

impl<'ink> AsBytesAndPtrs<'ink> for i64 {
    fn as_bytes_and_ptrs(&self, _: &IrTypeContext<'ink, '_>) -> Vec<BytesOrPtr<'ink>> {
        vec![bytemuck::cast_ref::<i64, [u8; 8]>(self).to_vec().into()]
    }
}

impl<'ink> AsBytesAndPtrs<'ink> for bool {
    fn as_bytes_and_ptrs(&self, _: &IrTypeContext<'ink, '_>) -> Vec<BytesOrPtr<'ink>> {
        vec![bytemuck::cast_ref::<bool, [u8; 1]>(self).to_vec().into()]
    }
}

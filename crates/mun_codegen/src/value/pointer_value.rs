use crate::value::{
    AddressableType, ConcreteValueType, IrTypeContext, IrValueContext, PointerValueType,
    SizedValueType, Value,
};
use inkwell::types::PointerType;
use inkwell::AddressSpace;

impl<'ink, T: PointerValueType<'ink>> ConcreteValueType<'ink> for *const T {
    type Value = inkwell::values::PointerValue<'ink>;
}
impl<'ink, T: PointerValueType<'ink>> ConcreteValueType<'ink> for *mut T {
    type Value = inkwell::values::PointerValue<'ink>;
}

impl<'ink, T: PointerValueType<'ink>> SizedValueType<'ink> for *const T {
    fn get_ir_type(context: &IrTypeContext<'ink, '_>) -> inkwell::types::PointerType<'ink> {
        T::get_ptr_type(context, None)
    }
}
impl<'ink, T: PointerValueType<'ink>> SizedValueType<'ink> for *mut T {
    fn get_ir_type(context: &IrTypeContext<'ink, '_>) -> inkwell::types::PointerType<'ink> {
        T::get_ptr_type(context, None)
    }
}
impl<'ink, T: PointerValueType<'ink>> PointerValueType<'ink> for *mut T {
    fn get_ptr_type(
        context: &IrTypeContext<'ink, '_>,
        address_space: Option<AddressSpace>,
    ) -> PointerType<'ink> {
        Self::get_ir_type(context).ptr_type(address_space.unwrap_or(AddressSpace::Generic))
    }
}
impl<'ink, T: PointerValueType<'ink>> PointerValueType<'ink> for *const T {
    fn get_ptr_type(
        context: &IrTypeContext<'ink, '_>,
        address_space: Option<AddressSpace>,
    ) -> PointerType<'ink> {
        Self::get_ir_type(context).ptr_type(address_space.unwrap_or(AddressSpace::Generic))
    }
}

impl<'ink, T: SizedValueType<'ink, Value = inkwell::values::PointerValue<'ink>>> Value<'ink, T> {
    /// Constructs a `null` pointer of type `T`
    pub fn null(context: &IrValueContext<'ink, '_, '_>) -> Self {
        Value::from_raw(T::get_ir_type(context.type_context).const_null())
    }
}

impl<'ink, T> AddressableType<'ink, *const T> for *const T where *const T: ConcreteValueType<'ink> {}

impl<'ink, T> AddressableType<'ink, *mut T> for *mut T where *mut T: ConcreteValueType<'ink> {}

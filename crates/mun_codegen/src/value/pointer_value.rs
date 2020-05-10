use crate::value::{
    ConcreteValueType, IrTypeContext, IrValueContext, PointerValueType, SizedValueType, Value,
};
use inkwell::types::PointerType;
use inkwell::AddressSpace;

impl<T: PointerValueType> ConcreteValueType for *const T {
    type Value = inkwell::values::PointerValue;
}
impl<T: PointerValueType> ConcreteValueType for *mut T {
    type Value = inkwell::values::PointerValue;
}

impl<T: PointerValueType> SizedValueType for *const T {
    fn get_ir_type(context: &IrTypeContext) -> inkwell::types::PointerType {
        T::get_ptr_type(context, None)
    }
}
impl<T: PointerValueType> SizedValueType for *mut T {
    fn get_ir_type(context: &IrTypeContext) -> inkwell::types::PointerType {
        T::get_ptr_type(context, None)
    }
}
impl<T: PointerValueType> PointerValueType for *mut T {
    fn get_ptr_type(context: &IrTypeContext, address_space: Option<AddressSpace>) -> PointerType {
        Self::get_ir_type(context).ptr_type(address_space.unwrap_or(AddressSpace::Generic))
    }
}
impl<T: PointerValueType> PointerValueType for *const T {
    fn get_ptr_type(context: &IrTypeContext, address_space: Option<AddressSpace>) -> PointerType {
        Self::get_ir_type(context).ptr_type(address_space.unwrap_or(AddressSpace::Generic))
    }
}

impl<T: SizedValueType<Value = inkwell::values::PointerValue>> Value<T> {
    /// Constructs a `null` pointer of type `T`
    pub fn null(context: &IrValueContext) -> Self {
        Value::from_raw(T::get_ir_type(context.type_context).const_null())
    }
}

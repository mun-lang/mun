use super::{
    AddressableType, AsBytesAndPtrs, BytesOrPtr, ConcreteValueType, HasConstValue, IrTypeContext,
    IrValueContext, PointerValueType, SizedValueType, Value,
};
use crate::value::ValueType;
use inkwell::{types::PointerType, AddressSpace};

impl<'ink> ConcreteValueType<'ink> for *const std::ffi::c_void {
    type Value = inkwell::values::PointerValue<'ink>;
}

impl<'ink> SizedValueType<'ink> for *const std::ffi::c_void {
    fn get_ir_type(
        context: &IrTypeContext<'ink, '_>,
    ) -> <<Self as ConcreteValueType<'ink>>::Value as ValueType<'ink>>::Type {
        context
            .context
            .ptr_sized_int_type(context.target_data, None)
            .ptr_type(AddressSpace::Generic)
    }
}

impl<'ink> PointerValueType<'ink> for *const std::ffi::c_void {
    fn get_ptr_type(
        context: &IrTypeContext<'ink, '_>,
        address_space: Option<AddressSpace>,
    ) -> PointerType<'ink> {
        Self::get_ir_type(context).ptr_type(address_space.unwrap_or(AddressSpace::Generic))
    }
}

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

impl<'ink, T> HasConstValue for Value<'ink, T>
where
    T: SizedValueType<'ink, Value = inkwell::values::PointerValue<'ink>>,
{
    fn has_const_value() -> bool {
        true
    }
}

impl<'ink, T> AsBytesAndPtrs<'ink> for Value<'ink, T>
where
    T: SizedValueType<'ink, Value = inkwell::values::PointerValue<'ink>>,
{
    fn as_bytes_and_ptrs(&self, _: &IrTypeContext<'ink, '_>) -> Vec<BytesOrPtr<'ink>> {
        vec![BytesOrPtr::UntypedPtr(self.value)]
    }
}

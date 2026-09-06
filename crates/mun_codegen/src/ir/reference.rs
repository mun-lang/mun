
use inkwell::{
    builder::Builder,
    types::{BasicType, BasicTypeEnum},
    values::{BasicValueEnum, PointerValue},
    AddressSpace,
};

use crate::ir::value::PlaceValue;

/// A stable runtime handle and the LLVM type of the heap object it references.
///
/// The handle is stored as `**T`: the runtime may replace `*T` while generated references remain
/// valid. Keeping `T` explicitly avoids recovering semantic information from LLVM pointer types.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) struct RuntimeReferenceValue<'ink> {
    pointer: PointerValue<'ink>,
    object_type: BasicTypeEnum<'ink>,
}

impl<'ink> RuntimeReferenceValue<'ink> {
    /// Associates a runtime handle with the object type it addresses.
    pub(crate) fn new(pointer: PointerValue<'ink>, object_type: BasicTypeEnum<'ink>) -> Self {
        debug_assert_eq!(
            pointer.get_type(),
            object_type
                .ptr_type(AddressSpace::default())
                .ptr_type(AddressSpace::default())
        );
        Self {
            pointer,
            object_type,
        }
    }


    /// Emits the runtime indirection and returns the resulting object place.
    pub(crate) fn get_data(&self, builder: &Builder<'ink>) -> PlaceValue<'ink> {
        let value_name = self.pointer.get_name().to_string_lossy();
        self.get_data_named(builder, &format!("{value_name}->data"))
    }

    /// Emits the runtime indirection with an explicit result name.
    pub(crate) fn get_data_named(
        &self,
        builder: &Builder<'ink>,
        name: &str,
    ) -> PlaceValue<'ink> {
        let pointer = builder.build_load(self.pointer, name).into_pointer_value();
        PlaceValue::new(pointer, self.object_type)
    }

}

impl<'ink> From<RuntimeReferenceValue<'ink>> for BasicValueEnum<'ink> {
    fn from(value: RuntimeReferenceValue<'ink>) -> Self {
        value.pointer.into()
    }
}

impl<'ink> From<RuntimeReferenceValue<'ink>> for PointerValue<'ink> {
    fn from(value: RuntimeReferenceValue<'ink>) -> Self {
        value.pointer
    }
}

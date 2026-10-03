//! Typed projections into heap-allocated Mun arrays.
//!
//! A runtime array handle points indirectly to `{ length, capacity,
//! first_element }`. This module keeps that aggregate type explicit so
//! projections do not depend on LLVM pointer element types.

use inkwell::{
    builder::Builder,
    types::{BasicTypeEnum, IntType, StructType},
    values::{BasicValueEnum, PointerValue},
};

use crate::ir::{reference::RuntimeReferenceValue, value::PlaceValue};

/// A runtime array handle paired with the concrete array aggregate type.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) struct RuntimeArrayValue<'ink> {
    reference: RuntimeReferenceValue<'ink>,
    array_type: StructType<'ink>,
}

impl<'ink> RuntimeArrayValue<'ink> {
    /// Associates a runtime handle with its concrete array aggregate type.
    pub(crate) fn new(pointer: PointerValue<'ink>, array_type: StructType<'ink>) -> Self {
        Self {
            reference: RuntimeReferenceValue::new(pointer, array_type.into()),
            array_type,
        }
    }

    fn get_array(&self, builder: &Builder<'ink>) -> PlaceValue<'ink> {
        self.reference.get_data(builder)
    }

    /// Projects the array length field.
    pub(crate) fn get_length(&self, builder: &Builder<'ink>) -> PlaceValue<'ink> {
        let array = self.get_array(builder);
        let pointer = array.pointer();
        let value_name = pointer.get_name().to_string_lossy();
        let pointer = builder
            .build_struct_gep(pointer, 0, &format!("{value_name}->length"))
            .expect("could not get `length` from array struct");
        PlaceValue::new(pointer, self.length_ty().into())
    }

    /// Projects the first array element.
    pub(crate) fn get_elements(&self, builder: &Builder<'ink>) -> PlaceValue<'ink> {
        let array = self.get_array(builder);
        let pointer = array.pointer();
        let value_name = pointer.get_name().to_string_lossy();
        let pointer = builder
            .build_struct_gep(pointer, 2, &format!("{value_name}->elements"))
            .expect("could not get `elements` from array struct");
        PlaceValue::new(pointer, self.element_ty())
    }

    /// Returns the type of the length field.
    pub(crate) fn length_ty(&self) -> IntType<'ink> {
        self.array_type
            .get_field_type_at_index(0)
            .expect("an array must have a length field")
            .into_int_type()
    }

    /// Returns the type of an array element.
    pub(crate) fn element_ty(&self) -> BasicTypeEnum<'ink> {
        self.array_type
            .get_field_type_at_index(2)
            .expect("an array must have an element field")
    }
}

impl<'ink> From<RuntimeArrayValue<'ink>> for BasicValueEnum<'ink> {
    fn from(value: RuntimeArrayValue<'ink>) -> Self {
        value.reference.into()
    }
}

impl<'ink> From<RuntimeArrayValue<'ink>> for PointerValue<'ink> {
    fn from(value: RuntimeArrayValue<'ink>) -> Self {
        value.reference.into()
    }
}

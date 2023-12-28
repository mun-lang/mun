//! Defines a helper struct `RuntimeArrayValue` which wraps an inkwell value and represents a pointer to
//! a heap allocated Mun array struct.
//!
//! Mun arrays are represented on the heap as:
//!
//! ```c
//! struct Obj {
//!     ArrayValueT *value;
//!     ...
//! }
//!
//! struct ArrayValueT {
//!     usize_t len;
//!     usize_t capacity;
//!     T elements[capacity];
//! }
//! ```

use crate::ir::reference::RuntimeReferenceValue;
use inkwell::builder::Builder;
use inkwell::types::{BasicTypeEnum, IntType, StructType};
use inkwell::values::{BasicValueEnum, IntValue, PointerValue};
use std::ffi::CStr;

/// A helper struct that wraps a [`PointerValue`] which points to an in memory Mun array value.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct RuntimeArrayValue<'ink>(RuntimeReferenceValue<'ink>);

impl<'ink> RuntimeArrayValue<'ink> {
    /// Constructs a new `RuntimeArrayValue` from a reference pointer to a specific array type.
    ///
    /// The pointer passed must be of type `**ArrayValueT`.
    pub fn from_ptr(ptr: PointerValue<'ink>, array_type: StructType<'ink>) -> Result<Self, String> {
        RuntimeReferenceValue::from_ptr(ptr, array_type).map(Self)
    }

    /// Constructs a new instance from an inkwell [`PointerValue`] without checking if this is actually
    /// a pointer to an array.
    pub unsafe fn from_ptr_unchecked(ptr: PointerValue<'ink>) -> Self {
        Self(RuntimeReferenceValue::from_ptr_unchecked(ptr))
    }

    /// Returns the name of the array
    pub fn get_name(&self) -> &CStr {
        self.0.get_name()
    }

    /// Generate code to get to the array value.
    fn get_array_ptr(&self, builder: &Builder<'ink>) -> PointerValue<'ink> {
        self.0.get_data_ptr(builder)
    }

    /// Generate code to fetch the length of the array.
    pub fn get_length_ptr(&self, builder: &Builder<'ink>) -> PointerValue<'ink> {
        let array_ptr = self.get_array_ptr(builder);
        let value_name = array_ptr.get_name().to_string_lossy();
        builder
            .build_struct_gep(array_ptr, 0, &format!("{}->length", &value_name))
            .expect("could not get `length` from array struct")
    }

    /// Generate code to fetch the capacity of the array.
    pub fn get_capacity(&self, builder: &Builder<'ink>) -> IntValue<'ink> {
        let array_ptr = self.get_array_ptr(builder);
        let value_name = array_ptr.get_name().to_string_lossy();
        let length_ptr = builder
            .build_struct_gep(array_ptr, 1, &format!("{}->capacity", &value_name))
            .expect("could not get `length` from array struct");
        builder
            .build_load(length_ptr, &format!("{}.capacity", &value_name))
            .into_int_value()
    }

    /// Generate code to a pointer to the elements stored in the array.
    pub fn get_elements(&self, builder: &Builder<'ink>) -> PointerValue<'ink> {
        let array_ptr = self.get_array_ptr(builder);
        let value_name = array_ptr.get_name().to_string_lossy();
        builder
            .build_struct_gep(array_ptr, 2, &format!("{}->elements", &value_name))
            .expect("could not get `elements` from array struct")
    }

    /// Returns the type of the `length` field
    pub fn length_ty(&self) -> IntType<'_> {
        self.array_data_ty()
            .get_field_type_at_index(0)
            .expect("an array must have a second field")
            .into_int_type()
    }

    /// Returns the type of the `length` field
    pub fn capacity_ty(&self) -> IntType<'_> {
        self.array_data_ty()
            .get_field_type_at_index(1)
            .expect("an array must have a second field")
            .into_int_type()
    }

    /// Returns the type of the elements stored in this array
    pub fn element_ty(&self) -> BasicTypeEnum<'ink> {
        self.array_data_ty()
            .get_field_type_at_index(2)
            .expect("an array must have a second field")
    }

    fn array_data_ty(&self) -> StructType<'ink> {
        self.0.get_type().into_struct_type()
    }
}

impl<'ink> From<RuntimeArrayValue<'ink>> for BasicValueEnum<'ink> {
    fn from(value: RuntimeArrayValue<'ink>) -> Self {
        value.0.into()
    }
}

impl<'ink> From<RuntimeArrayValue<'ink>> for PointerValue<'ink> {
    fn from(value: RuntimeArrayValue<'ink>) -> Self {
        value.0.into()
    }
}

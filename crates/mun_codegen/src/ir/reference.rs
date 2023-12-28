use inkwell::builder::Builder;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValueEnum, PointerValue};
use inkwell::AddressSpace;
use std::ffi::CStr;

/// A helper struct that wraps an object on the heap.
///
/// Objects on the heap are represented as an indirection. The stored pointer points to an object
/// on the heap where the first field points to the actual data of the object:
///
/// ```c
/// struct Obj {
///     ObjectData *data;
///     ...
/// }
/// ```
///
/// This enables the runtime to modify the contents of the object without having to modify the
/// references that point to it.
///
/// The `RuntimeReferenceValue` stores the indirection as `**T` (a pointer to a pointer to `T`),
/// where T is the type of the object stored on the heap.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct RuntimeReferenceValue<'ink>(PointerValue<'ink>);

impl<'ink> RuntimeReferenceValue<'ink> {
    /// Constructs a new `RuntimeReferenceValue` from a reference pointer to a specific type.
    ///
    /// The pointer passed must be of type `**T`.
    pub fn from_ptr(
        ptr: PointerValue<'ink>,
        object_type: impl BasicType<'ink>,
    ) -> Result<Self, String> {
        let reference_type = object_type
            .ptr_type(AddressSpace::default())
            .ptr_type(AddressSpace::default());
        if ptr.get_type() == reference_type {
            Ok(Self(ptr))
        } else {
            Err(format!(
                "expected pointer of type {}, got {}",
                reference_type.print_to_string().to_string_lossy(),
                ptr.get_type().print_to_string().to_string_lossy()
            ))
        }
    }

    /// Constructs a new instance from an inkwell `PointerValue` without checking if this is actually
    /// a pointer to an object on the heap.
    pub unsafe fn from_ptr_unchecked(ptr: PointerValue<'ink>) -> Self {
        Self(ptr)
    }

    /// Returns the name of the inkwell value
    pub fn get_name(&self) -> &CStr {
        self.0.get_name()
    }

    /// Generates code to dereference the reference to get to the data of the reference.
    pub fn get_data_ptr(&self, builder: &Builder<'ink>) -> PointerValue<'ink> {
        let value_name = self.0.get_name().to_string_lossy();

        // Dereference the pointer to get the pointer to the data
        //
        // ```c
        // data_ptr:*const T: = *data_ptr_ptr;
        // ```
        builder
            .build_load(self.0, &format!("{}->data", &value_name))
            .into_pointer_value()
    }

    /// Returns the type of the object this instance points to
    pub fn get_type(&self) -> BasicTypeEnum<'ink> {
        self.0
            .get_type()
            .get_element_type()
            .into_pointer_type()
            .get_element_type()
            .try_into()
            .expect("could not convert reference type to basic type")
    }
}

impl<'ink> From<RuntimeReferenceValue<'ink>> for BasicValueEnum<'ink> {
    fn from(value: RuntimeReferenceValue<'ink>) -> Self {
        value.0.into()
    }
}

impl<'ink> From<RuntimeReferenceValue<'ink>> for PointerValue<'ink> {
    fn from(value: RuntimeReferenceValue<'ink>) -> Self {
        value.0
    }
}

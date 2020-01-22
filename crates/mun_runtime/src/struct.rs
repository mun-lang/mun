use crate::{
    marshal::MarshalInto,
    reflection::{
        equals_argument_type, equals_return_type, ArgumentReflection, ReturnTypeReflection,
    },
};
use abi::{StructInfo, TypeInfo};
use std::mem;

/// Represents a Mun struct pointer.
///
/// A byte pointer is used to make pointer arithmetic easier.
#[repr(transparent)]
#[derive(Clone)]
pub struct RawStruct(*mut u8);

/// Type-agnostic wrapper for interoperability with a Mun struct.
/// TODO: Handle destruction of `struct(value)`
#[derive(Clone)]
pub struct Struct {
    raw: RawStruct,
    info: StructInfo,
}

impl Struct {
    /// Creates a struct that wraps a raw Mun struct.
    ///
    /// The provided [`TypeInfo`] must be for a struct type.
    fn new(type_info: &TypeInfo, raw: RawStruct) -> Self {
        assert!(type_info.group.is_struct());

        Self {
            raw,
            info: type_info.as_struct().unwrap().clone(),
        }
    }

    /// Consumes the `Struct`, returning a raw Mun struct.
    pub fn into_raw(self) -> RawStruct {
        self.raw
    }

    /// Retrieves the value of the field corresponding to the specified `field_name`.
    pub fn get<T: ReturnTypeReflection>(&self, field_name: &str) -> Result<T, String> {
        let field_idx = StructInfo::find_field_index(&self.info, field_name)?;
        let field_type = unsafe { self.info.field_types().get_unchecked(field_idx) };
        equals_return_type::<T>(&field_type).map_err(|(expected, found)| {
            format!(
                "Mismatched types for `{}::{}`. Expected: `{}`. Found: `{}`.",
                self.info.name(),
                field_name,
                expected,
                found,
            )
        })?;

        let field_value = unsafe {
            // If we found the `field_idx`, we are guaranteed to also have the `field_offset`
            let offset = *self.info.field_offsets().get_unchecked(field_idx);
            // self.ptr is never null
            // TODO: The unsafe `read` fn could be avoided by adding the `Clone` bound on
            // `T::Marshalled`, but its only available on nightly:
            // `ReturnTypeReflection<Marshalled: Clone>`
            self.raw
                .0
                .add(offset as usize)
                .cast::<T::Marshalled>()
                .read()
        };
        Ok(field_value.marshal_into(Some(*field_type)))
    }

    /// Replaces the value of the field corresponding to the specified `field_name` and returns the
    /// old value.
    pub fn replace<T: ArgumentReflection>(
        &mut self,
        field_name: &str,
        value: T,
    ) -> Result<T, String> {
        let field_idx = StructInfo::find_field_index(&self.info, field_name)?;
        let field_type = unsafe { self.info.field_types().get_unchecked(field_idx) };
        equals_argument_type(&field_type, &value).map_err(|(expected, found)| {
            format!(
                "Mismatched types for `{}::{}`. Expected: `{}`. Found: `{}`.",
                self.info.name(),
                field_name,
                expected,
                found,
            )
        })?;

        let mut marshalled: T::Marshalled = value.marshal();
        let ptr = unsafe {
            // If we found the `field_idx`, we are guaranteed to also have the `field_offset`
            let offset = *self.info.field_offsets().get_unchecked(field_idx);
            // self.ptr is never null
            &mut *self.raw.0.add(offset as usize).cast::<T::Marshalled>()
        };
        mem::swap(&mut marshalled, ptr);
        Ok(marshalled.marshal_into(Some(*field_type)))
    }

    /// Sets the value of the field corresponding to the specified `field_name`.
    pub fn set<T: ArgumentReflection>(&mut self, field_name: &str, value: T) -> Result<(), String> {
        let field_idx = StructInfo::find_field_index(&self.info, field_name)?;
        let field_type = unsafe { self.info.field_types().get_unchecked(field_idx) };
        equals_argument_type(&field_type, &value).map_err(|(expected, found)| {
            format!(
                "Mismatched types for `{}::{}`. Expected: `{}`. Found: `{}`.",
                self.info.name(),
                field_name,
                expected,
                found,
            )
        })?;

        unsafe {
            // If we found the `field_idx`, we are guaranteed to also have the `field_offset`
            let offset = *self.info.field_offsets().get_unchecked(field_idx);
            // self.ptr is never null
            *self.raw.0.add(offset as usize).cast::<T::Marshalled>() = value.marshal();
        }
        Ok(())
    }
}

impl ArgumentReflection for Struct {
    type Marshalled = RawStruct;

    fn type_name(&self) -> &str {
        self.info.name()
    }

    fn marshal(self) -> Self::Marshalled {
        self.raw
    }
}

impl ReturnTypeReflection for Struct {
    type Marshalled = RawStruct;

    fn type_name() -> &'static str {
        "struct"
    }
}

impl MarshalInto<Struct> for RawStruct {
    fn marshal_into(self, type_info: Option<&TypeInfo>) -> Struct {
        // `type_info` is only `None` for the `()` type
        Struct::new(type_info.unwrap(), self)
    }
}

use crate::{
    marshal::MarshalInto,
    reflection::{ArgumentReflection, ReturnTypeReflection},
    Runtime,
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
    pub fn new(runtime: &Runtime, type_info: &TypeInfo, raw: RawStruct) -> Result<Struct, String> {
        let struct_info = runtime.get_struct_info(type_info.name()).ok_or_else(|| {
            format!(
                "Could not find information for struct `{}`.",
                type_info.name()
            )
        })?;

        Ok(Self {
            raw,
            info: struct_info.clone(),
        })
    }

    /// Consumes the `Struct`, returning a raw Mun struct.
    pub fn into_raw(self) -> RawStruct {
        self.raw
    }

    /// Retrieves the value of the field corresponding to the specified `field_name`.
    pub fn get<T: ReturnTypeReflection>(&self, field_name: &str) -> Result<&T, String> {
        let field_idx = self
            .info
            .field_names()
            .enumerate()
            .find(|(_, name)| *name == field_name)
            .map(|(idx, _)| idx)
            .ok_or_else(|| {
                format!(
                    "Struct `{}` does not contain field `{}`.",
                    self.info.name(),
                    field_name
                )
            })?;

        let field_type = unsafe { self.info.field_types().get_unchecked(field_idx) };
        if T::type_guid() != field_type.guid {
            return Err(format!(
                "Mismatched types for `{}::{}`. Expected: `{}`. Found: `{}`.",
                self.info.name(),
                field_name,
                field_type.name(),
                T::type_name()
            ));
        }

        unsafe {
            // If we found the `field_idx`, we are guaranteed to also have the `field_offset`
            let offset = *self.info.field_offsets().get_unchecked(field_idx);
            // self.ptr is never null
            Ok(&*self.raw.0.add(offset as usize).cast::<T>())
        }
    }

    /// Replaces the value of the field corresponding to the specified `field_name` and returns the
    /// old value.
    pub fn replace<T: ArgumentReflection>(
        &mut self,
        field_name: &str,
        mut value: T,
    ) -> Result<T, String> {
        let field_idx = self
            .info
            .field_names()
            .enumerate()
            .find(|(_, name)| *name == field_name)
            .map(|(idx, _)| idx)
            .ok_or_else(|| {
                format!(
                    "Struct `{}` does not contain field `{}`.",
                    self.info.name(),
                    field_name
                )
            })?;

        let field_type = unsafe { self.info.field_types().get_unchecked(field_idx) };
        if value.type_guid() != field_type.guid {
            return Err(format!(
                "Mismatched types for `{}::{}`. Expected: `{}`. Found: `{}`.",
                self.info.name(),
                field_name,
                field_type.name(),
                value.type_name()
            ));
        }

        let ptr = unsafe {
            // If we found the `field_idx`, we are guaranteed to also have the `field_offset`
            let offset = *self.info.field_offsets().get_unchecked(field_idx);
            // self.ptr is never null
            &mut *self.raw.0.add(offset as usize).cast::<T>()
        };
        mem::swap(&mut value, ptr);
        Ok(value)
    }

    /// Sets the value of the field corresponding to the specified `field_name`.
    pub fn set<T: ArgumentReflection>(&mut self, field_name: &str, value: T) -> Result<(), String> {
        let field_idx = self
            .info
            .field_names()
            .enumerate()
            .find(|(_, name)| *name == field_name)
            .map(|(idx, _)| idx)
            .ok_or_else(|| {
                format!(
                    "Struct `{}` does not contain field `{}`.",
                    self.info.name(),
                    field_name
                )
            })?;

        let field_type = unsafe { self.info.field_types().get_unchecked(field_idx) };
        if value.type_guid() != field_type.guid {
            return Err(format!(
                "Mismatched types for `{}::{}`. Expected: `{}`. Found: `{}`.",
                self.info.name(),
                field_name,
                field_type.name(),
                value.type_name()
            ));
        }

        unsafe {
            // If we found the `field_idx`, we are guaranteed to also have the `field_offset`
            let offset = *self.info.field_offsets().get_unchecked(field_idx);
            // self.ptr is never null
            *self.raw.0.add(offset as usize).cast::<T>() = value;
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
    fn marshal_into(self, runtime: &Runtime, type_info: Option<&TypeInfo>) -> Struct {
        // `type_info` is only `None` for the `()` type
        // the `StructInfo` for this type must have been loaded for a function to return its type
        Struct::new(runtime, type_info.unwrap(), self).unwrap()
    }
}

use crate::{
    marshal::Marshal,
    reflection::{
        equals_argument_type, equals_return_type, ArgumentReflection, ReturnTypeReflection,
    },
    Runtime,
};
use abi::{StructInfo, StructMemoryKind, TypeInfo};
use std::cell::RefCell;
use std::ptr::{self, NonNull};
use std::rc::Rc;

/// Represents a Mun struct pointer.
///
/// A byte pointer is used to make pointer arithmetic easier.
#[repr(transparent)]
#[derive(Clone)]
pub struct RawStruct(*mut u8);

/// Type-agnostic wrapper for interoperability with a Mun struct.
/// TODO: Handle destruction of `struct(value)`
pub struct StructRef {
    runtime: Rc<RefCell<Runtime>>,
    raw: RawStruct,
    info: StructInfo,
}

impl StructRef {
    /// Creates a struct that wraps a raw Mun struct.
    ///
    /// The provided [`TypeInfo`] must be for a struct type.
    fn new(runtime: Rc<RefCell<Runtime>>, type_info: &TypeInfo, raw: RawStruct) -> StructRef {
        assert!(type_info.group.is_struct());

        Self {
            runtime,
            raw,
            info: type_info.as_struct().unwrap().clone(),
        }
    }

    /// Consumes the `Struct`, returning a raw Mun struct.
    pub fn into_raw(self) -> RawStruct {
        self.raw
    }

    /// Retrieves its struct information.
    pub fn info(&self) -> &StructInfo {
        &self.info
    }

    ///
    ///
    /// # Safety
    ///
    ///
    unsafe fn offset_unchecked<T>(&self, field_idx: usize) -> NonNull<T> {
        let offset = *self.info.field_offsets().get_unchecked(field_idx);
        // self.raw is never null
        NonNull::new_unchecked(self.raw.0.add(offset as usize)).cast::<T>()
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

        // If we found the `field_idx`, we are guaranteed to also have the `field_offset`
        let field_ptr = unsafe { self.offset_unchecked::<T::Marshalled>(field_idx) };
        Ok(Marshal::marshal_from_ptr(
            field_ptr,
            self.runtime.clone(),
            Some(*field_type),
        ))
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

        let field_ptr = unsafe { self.offset_unchecked::<T::Marshalled>(field_idx) };
        let old = Marshal::marshal_from_ptr(field_ptr, self.runtime.clone(), Some(*field_type));
        Marshal::marshal_to_ptr(value.marshal(), field_ptr, Some(*field_type));
        Ok(old)
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

        let field_ptr = unsafe { self.offset_unchecked::<T::Marshalled>(field_idx) };
        Marshal::marshal_to_ptr(value.marshal(), field_ptr, Some(*field_type));
        Ok(())
    }
}

impl ArgumentReflection for StructRef {
    type Marshalled = RawStruct;

    fn type_name(&self) -> &str {
        self.info.name()
    }

    fn marshal(self) -> Self::Marshalled {
        self.raw
    }
}

impl ReturnTypeReflection for StructRef {
    type Marshalled = RawStruct;

    fn type_name() -> &'static str {
        "struct"
    }
}

impl Marshal<StructRef> for RawStruct {
    fn marshal_value(
        self,
        runtime: Rc<RefCell<Runtime>>,
        type_info: Option<&TypeInfo>,
    ) -> StructRef {
        // `type_info` is only `None` for the `()` type
        StructRef::new(runtime, type_info.unwrap(), self)
    }

    fn marshal_from_ptr(
        ptr: NonNull<Self>,
        runtime: Rc<RefCell<Runtime>>,
        type_info: Option<&TypeInfo>,
    ) -> StructRef {
        // `type_info` is only `None` for the `()` type
        let type_info = type_info.unwrap();

        let struct_info = type_info.as_struct().unwrap();
        let ptr = if struct_info.memory_kind == StructMemoryKind::Value {
            ptr.cast::<u8>().as_ptr() as *const _
        } else {
            unsafe { ptr.as_ref() }.0 as *const _
        };

        // Clone the struct using the runtime's intrinsic
        let cloned_ptr = invoke_fn!(runtime.clone(), "clone", ptr, type_info as *const _).unwrap();
        StructRef::new(runtime, type_info, RawStruct(cloned_ptr))
    }

    fn marshal_to_ptr(value: RawStruct, mut ptr: NonNull<Self>, type_info: Option<&TypeInfo>) {
        // `type_info` is only `None` for the `()` type
        let type_info = type_info.unwrap();

        let struct_info = type_info.as_struct().unwrap();
        if struct_info.memory_kind == StructMemoryKind::Value {
            let dest = ptr.cast::<u8>().as_ptr();
            let size = struct_info.field_offsets().last().cloned().unwrap_or(0)
                + struct_info.field_sizes().last().cloned().unwrap_or(0);
            unsafe { ptr::copy_nonoverlapping(value.0, dest, size as usize) };
        } else {
            unsafe { *ptr.as_mut() = value };
        }
    }
}

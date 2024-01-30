#[cfg(test)]
mod test;

use itertools::Itertools;
use mun_abi as abi;
use mun_codegen_macros::AsValue;

use crate::value::{
    AsValue, BytesOrPtr, Global, IrTypeContext, IrValueContext, SizedValueType, TransparentValue,
    Value,
};

impl<'ink> TransparentValue<'ink> for abi::Guid {
    type Target = [u8; 16];

    fn as_target_value(&self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, Self::Target> {
        self.0.as_value(context)
    }

    fn as_bytes_and_ptrs(&self, _: &IrTypeContext<'ink, '_>) -> Vec<BytesOrPtr<'ink>> {
        vec![self.0.to_vec().into()]
    }
}

impl<'ink> TransparentValue<'ink> for abi::Privacy {
    type Target = u8;

    fn as_target_value(&self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, Self::Target> {
        (*self as u8).as_value(context)
    }

    fn as_bytes_and_ptrs(&self, _: &IrTypeContext<'ink, '_>) -> Vec<BytesOrPtr<'ink>> {
        vec![vec![*self as u8].into()]
    }
}

impl<'ink> TransparentValue<'ink> for abi::StructMemoryKind {
    type Target = u8;

    fn as_target_value(&self, context: &IrValueContext<'ink, '_, '_>) -> Value<'ink, Self::Target> {
        (*self as u8).as_value(context)
    }

    fn as_bytes_and_ptrs(&self, _: &IrTypeContext<'ink, '_>) -> Vec<BytesOrPtr<'ink>> {
        vec![vec![*self as u8].into()]
    }
}

#[derive(AsValue)]
#[repr(u8)]
pub enum TypeId<'ink> {
    Concrete(abi::Guid),
    Pointer(PointerTypeId<'ink>),
    Array(ArrayTypeId<'ink>),
}

#[derive(AsValue)]
pub struct PointerTypeId<'ink> {
    pub pointee: Global<'ink, TypeId<'ink>>,
    pub mutable: bool,
}

#[derive(AsValue)]
pub struct ArrayTypeId<'ink> {
    pub element: Global<'ink, TypeId<'ink>>,
}

#[derive(AsValue)]
pub struct TypeDefinition<'ink> {
    pub name: Value<'ink, *const u8>,
    pub size_in_bits: u32,
    pub alignment: u8,
    pub data: TypeDefinitionData<'ink>,
}

#[derive(AsValue)]
#[repr(u8)]
pub enum TypeDefinitionData<'ink> {
    Struct(StructDefinition<'ink>),
}

#[derive(AsValue)]
pub struct FunctionSignature<'ink> {
    pub arg_types: Value<'ink, *const TypeId<'ink>>,
    pub return_type: TypeId<'ink>,
    pub num_arg_types: u16,
}

#[derive(AsValue)]
pub struct FunctionPrototype<'ink> {
    pub name: Value<'ink, *const u8>,
    pub signature: FunctionSignature<'ink>,
}

#[derive(AsValue)]
pub struct FunctionDefinition<'ink> {
    pub prototype: FunctionPrototype<'ink>,
    pub fn_ptr: Value<'ink, *const fn()>,
}

#[derive(AsValue)]
pub struct StructDefinition<'ink> {
    pub guid: abi::Guid,
    pub field_names: Value<'ink, *const *const u8>,
    pub field_types: Value<'ink, *const TypeId<'ink>>,
    pub field_offsets: Value<'ink, *const u16>,
    pub num_fields: u16,
    pub memory_kind: abi::StructMemoryKind,
}

#[derive(AsValue)]
pub struct ModuleInfo<'ink> {
    pub path: Value<'ink, *const u8>,
    pub functions: Value<'ink, *const FunctionDefinition<'ink>>,
    pub types: Value<'ink, *const TypeDefinition<'ink>>,
    pub num_functions: u32,
    pub num_types: u32,
}

#[derive(AsValue)]
pub struct DispatchTable<'ink> {
    pub prototypes: Value<'ink, *const FunctionPrototype<'ink>>,
    pub fn_ptrs: Value<'ink, *mut *const fn()>,
    pub num_entries: u32,
}

#[derive(AsValue)]
pub struct TypeLut<'ink> {
    pub type_ids: Value<'ink, *const TypeId<'ink>>,
    pub type_ptrs: Value<'ink, *mut *const std::ffi::c_void>,
    pub type_names: Value<'ink, *const *const u8>,
    pub num_entries: u32,
}

#[derive(AsValue)]
pub struct AssemblyInfo<'ink> {
    pub symbols: ModuleInfo<'ink>,
    pub dispatch_table: DispatchTable<'ink>,
    pub type_lut: TypeLut<'ink>,
    pub dependencies: Value<'ink, *const *const u8>,
    pub num_dependencies: u32,
}

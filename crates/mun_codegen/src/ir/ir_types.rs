use crate::value::{AsValue, IrValueContext, SizedValueType, TransparentValue, Value};

impl TransparentValue for abi::Guid {
    type Target = [u8; 16];

    fn as_target_value(&self, context: &IrValueContext) -> Value<Self::Target> {
        self.b.as_value(context)
    }
}

impl TransparentValue for abi::Privacy {
    type Target = u8;

    fn as_target_value(&self, context: &IrValueContext) -> Value<Self::Target> {
        (*self as u8).as_value(context)
    }
}

impl TransparentValue for abi::TypeGroup {
    type Target = u8;

    fn as_target_value(&self, context: &IrValueContext) -> Value<Self::Target> {
        (*self as u8).as_value(context)
    }
}

impl TransparentValue for abi::StructMemoryKind {
    type Target = u8;

    fn as_target_value(&self, context: &IrValueContext) -> Value<Self::Target> {
        (self.clone() as u8).as_value(context)
    }
}

#[derive(AsValue)]
#[ir_name = "struct.MunTypeInfo"]
pub struct TypeInfo {
    guid: abi::Guid,
    name: Value<*const u8>,
    size_in_bits: u32,
    alignment: u8,
    type_group: abi::TypeGroup,
}

#[derive(AsValue)]
#[ir_name = "struct.MunFunctionSignature"]
pub struct FunctionSignature {
    arg_types: Value<*const *const TypeInfo>,
    return_type: Value<*const TypeInfo>,
    num_arg_types: u16,
}

#[derive(AsValue)]
#[ir_name = "struct.MunFunctionPrototype"]
pub struct FunctionPrototype {
    name: Value<*const u8>,
    signature: FunctionSignature,
}

#[derive(AsValue)]
#[ir_name = "struct.MunFunctionDefinition"]
pub struct FunctionDefinition {
    prototype: FunctionPrototype,
    fn_ptr: Value<*const fn()>,
}

#[derive(AsValue)]
#[ir_name = "struct.MunStructInfo"]
pub struct StructInfo {
    field_names: Value<*const *const u8>,
    field_types: Value<*const *const TypeInfo>,
    field_offsets: Value<*const u16>,
    num_fields: u16,
    memory_kind: abi::StructMemoryKind,
}

#[derive(AsValue)]
#[ir_name = "struct.MunModuleInfo"]
pub struct ModuleInfo {
    path: Value<*const u8>,
    functions: Value<*const FunctionDefinition>,
    num_functions: u32,
    types: Value<*const *const TypeInfo>,
    num_types: u32,
}

#[derive(AsValue)]
#[ir_name = "struct.MunDispatchTable"]
pub struct DispatchTable {
    signatures: Value<*const FunctionSignature>,
    fn_ptrs: Value<*const *mut fn()>,
    num_entries: u32,
}

#[derive(AsValue)]
#[ir_name = "struct.MunAssemblyInfo"]
pub struct AssemblyInfo {
    symbols: ModuleInfo,
    dispatch_table: DispatchTable,
    dependencies: Value<*const *const u8>,
    num_dependencies: u32,
}

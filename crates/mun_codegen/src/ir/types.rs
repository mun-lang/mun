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

#[derive(AsValue, TestIsAbiCompatible)]
#[ir_name = "struct.MunTypeInfo"]
#[abi_type(abi::TypeInfo)]
pub struct TypeInfo {
    pub guid: abi::Guid,
    pub name: Value<*const u8>,
    pub size_in_bits: u32,
    pub alignment: u8,
    pub group: abi::TypeGroup,
}

#[derive(AsValue, TestIsAbiCompatible)]
#[ir_name = "struct.MunFunctionSignature"]
#[abi_type(abi::FunctionSignature)]
pub struct FunctionSignature {
    pub arg_types: Value<*const *const TypeInfo>,
    pub return_type: Value<*const TypeInfo>,
    pub num_arg_types: u16,
}

#[derive(AsValue, TestIsAbiCompatible)]
#[ir_name = "struct.MunFunctionPrototype"]
#[abi_type(abi::FunctionPrototype)]
pub struct FunctionPrototype {
    pub name: Value<*const u8>,
    pub signature: FunctionSignature,
}

#[derive(AsValue, TestIsAbiCompatible)]
#[ir_name = "struct.MunFunctionDefinition"]
#[abi_type(abi::FunctionDefinition)]
pub struct FunctionDefinition {
    pub prototype: FunctionPrototype,
    pub fn_ptr: Value<*const fn()>,
}

#[derive(AsValue, TestIsAbiCompatible)]
#[ir_name = "struct.MunStructInfo"]
#[abi_type(abi::StructInfo)]
pub struct StructInfo {
    pub field_names: Value<*const *const u8>,
    pub field_types: Value<*const *const TypeInfo>,
    pub field_offsets: Value<*const u16>,
    pub num_fields: u16,
    pub memory_kind: abi::StructMemoryKind,
}

#[derive(AsValue, TestIsAbiCompatible)]
#[ir_name = "struct.MunModuleInfo"]
#[abi_type(abi::ModuleInfo)]
pub struct ModuleInfo {
    pub path: Value<*const u8>,
    pub functions: Value<*const FunctionDefinition>,
    pub num_functions: u32,
    pub types: Value<*const *const TypeInfo>,
    pub num_types: u32,
}

#[derive(AsValue, TestIsAbiCompatible)]
#[ir_name = "struct.MunDispatchTable"]
#[abi_type(abi::DispatchTable)]
pub struct DispatchTable {
    pub prototypes: Value<*const FunctionPrototype>,
    pub fn_ptrs: Value<*mut *const fn()>,
    pub num_entries: u32,
}

#[derive(AsValue, TestIsAbiCompatible)]
#[ir_name = "struct.MunAssemblyInfo"]
#[abi_type(abi::AssemblyInfo)]
pub struct AssemblyInfo {
    pub symbols: ModuleInfo,
    pub dispatch_table: DispatchTable,
    pub dependencies: Value<*const *const u8>,
    pub num_dependencies: u32,
}

#[cfg(test)]
mod test;

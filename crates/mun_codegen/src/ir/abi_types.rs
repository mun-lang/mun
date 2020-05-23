use super::ir_types as ir;
use crate::value::{IrTypeContext, Value};
use inkwell::types::{ArrayType, IntType, StructType};

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct AbiTypes {
    pub guid_type: ArrayType,
    pub type_group_type: IntType,
    pub privacy_type: IntType,
    pub type_info_type: StructType,
    pub function_signature_type: StructType,
    pub function_prototype_type: StructType,
    pub function_definition_type: StructType,
    pub struct_info_type: StructType,
    pub module_info_type: StructType,
    pub dispatch_table_type: StructType,
    pub assembly_info_type: StructType,
}

/// Returns an `AbiTypes` struct that contains references to all LLVM ABI types.
pub(crate) fn gen_abi_types(context: &IrTypeContext) -> AbiTypes {
    let guid_type = Value::<abi::Guid>::get_ir_type(context);
    let type_group_type = Value::<abi::TypeGroup>::get_ir_type(context);
    let privacy_type = Value::<abi::Privacy>::get_ir_type(context);
    let type_info_type = Value::<ir::TypeInfo>::get_ir_type(context);
    let function_signature_type = Value::<ir::FunctionSignature>::get_ir_type(context);
    let function_prototype_type = Value::<ir::FunctionPrototype>::get_ir_type(context);
    let function_definition_type = Value::<ir::FunctionDefinition>::get_ir_type(context);
    let struct_info_type = Value::<ir::StructInfo>::get_ir_type(context);
    let module_info_type = Value::<ir::ModuleInfo>::get_ir_type(context);
    let dispatch_table_type = Value::<ir::DispatchTable>::get_ir_type(context);
    let assembly_info_type = Value::<ir::AssemblyInfo>::get_ir_type(context);

    AbiTypes {
        guid_type,
        type_group_type,
        privacy_type,
        type_info_type,
        function_signature_type,
        function_prototype_type,
        function_definition_type,
        struct_info_type,
        module_info_type,
        dispatch_table_type,
        assembly_info_type,
    }
}

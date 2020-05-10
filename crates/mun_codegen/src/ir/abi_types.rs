use super::ir_types as ir;
use crate::value::{IrTypeContext, Value};
use inkwell::types::{ArrayType, IntType, StructType};
use inkwell::AddressSpace;

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
    let str_type = context.context.i8_type().ptr_type(AddressSpace::Const);

    // Construct the `MunGuid` type
    let guid_type = Value::<abi::Guid>::get_ir_type(context);

    // Construct the `MunTypeGroup` type
    let type_group_type = Value::<abi::TypeGroup>::get_ir_type(context);

    // Construct the `MunPrivacy` enum
    let privacy_type = Value::<abi::Privacy>::get_ir_type(context);

    // Construct the `MunTypeInfo` struct
    // let type_info_type = context.context.opaque_struct_type("struct.MunTypeInfo");
    // type_info_type.set_body(
    //     &[
    //         guid_type.into(),          // guid
    //         str_type.into(),           // name
    //         context.context.i32_type().into(), // size_in_bits
    //         context.context.i8_type().into(),  // alignment
    //         type_group_type.into(),    // group
    //     ],
    //     false,
    // );
    let type_info_type = Value::<ir::TypeInfo>::get_ir_type(context);

    let type_info_ptr_type = type_info_type.ptr_type(AddressSpace::Const);

    // Construct the `MunFunctionSignature` type
    // let function_signature_type = context.context.opaque_struct_type("struct.MunFunctionSignature");
    // function_signature_type.set_body(
    //     &[
    //         type_info_ptr_type.ptr_type(AddressSpace::Const).into(), // arg_types
    //         type_info_ptr_type.into(),                               // return_type
    //         context.context.i16_type().into(),                               // num_arg_types
    //     ],
    //     false,
    // );
    let function_signature_type = Value::<ir::FunctionSignature>::get_ir_type(context);

    // Construct the `MunFunctionSignature` type
    let function_prototype_type = context
        .context
        .opaque_struct_type("struct.MunFunctionPrototype");
    function_prototype_type.set_body(
        &[
            str_type.into(),                // name
            function_signature_type.into(), // signature
        ],
        false,
    );

    // Construct the `MunFunctionDefinition` struct
    let function_definition_type = context
        .context
        .opaque_struct_type("struct.MunFunctionDefinition");
    function_definition_type.set_body(
        &[
            function_prototype_type.into(), // prototype
            context
                .context
                .void_type()
                .fn_type(&[], false)
                .ptr_type(AddressSpace::Const)
                .into(), // fn_ptr
        ],
        false,
    );

    // Construct the `MunStructInfo` struct
    let struct_info_type = context.context.opaque_struct_type("struct.MunStructInfo");
    struct_info_type.set_body(
        &[
            str_type.ptr_type(AddressSpace::Const).into(), // field_names
            type_info_ptr_type.ptr_type(AddressSpace::Const).into(), // field_types
            context
                .context
                .i16_type()
                .ptr_type(AddressSpace::Const)
                .into(), // field_offsets
            context.context.i16_type().into(),             // num_fields
            context.context.i8_type().into(),              // memory_kind
        ],
        false,
    );

    // Construct the `MunModuleInfo` struct
    let module_info_type = context.context.opaque_struct_type("struct.MunModuleInfo");
    module_info_type.set_body(
        &[
            str_type.into(), // path
            function_definition_type
                .ptr_type(AddressSpace::Const)
                .into(), // functions
            context.context.i32_type().into(), // num_functions
            type_info_ptr_type.ptr_type(AddressSpace::Const).into(), // types
            context.context.i32_type().into(), // num_types
        ],
        false,
    );

    // Construct the `MunDispatchTable` struct
    let dispatch_table_type = context
        .context
        .opaque_struct_type("struct.MunDispatchTable");
    dispatch_table_type.set_body(
        &[
            function_signature_type.ptr_type(AddressSpace::Const).into(), // signatures
            context
                .context
                .void_type()
                .fn_type(&[], false)
                .ptr_type(AddressSpace::Generic)
                .ptr_type(AddressSpace::Const)
                .into(), // fn_ptrs
            context.context.i32_type().into(),                            // num_entries
        ],
        false,
    );

    // Construct the `MunAssemblyInfo` struct
    let assembly_info_type = context.context.opaque_struct_type("struct.MunAssemblyInfo");
    assembly_info_type.set_body(
        &[
            module_info_type.into(),
            dispatch_table_type.into(),
            str_type.ptr_type(AddressSpace::Const).into(),
            context.context.i32_type().into(),
        ],
        false,
    );

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

use inkwell::context::ContextRef;
use inkwell::types::{ArrayType, IntType, StructType};
use inkwell::AddressSpace;

pub(super) struct AbiTypes {
    pub guid_type: ArrayType,
    pub type_group_type: IntType,
    pub privacy_type: IntType,
    pub type_info_type: StructType,
    pub function_signature_type: StructType,
    pub function_info_type: StructType,
    pub struct_info_type: StructType,
    pub module_info_type: StructType,
    pub dispatch_table_type: StructType,
    pub assembly_info_type: StructType,
}

/// Returns an `AbiTypes` struct that contains references to all LLVM ABI types.
pub(super) fn gen_abi_types(context: ContextRef) -> AbiTypes {
    let str_type = context.i8_type().ptr_type(AddressSpace::Const);

    // Construct the `MunGuid` type
    let guid_type = context.i8_type().array_type(16);

    // Construct the `MunTypeGroup` type
    let type_group_type = context.i8_type();

    // Construct the `MunPrivacy` enum
    let privacy_type = context.i8_type();

    // Construct the `MunTypeInfo` struct
    let type_info_type = context.opaque_struct_type("struct.MunTypeInfo");
    type_info_type.set_body(
        &[
            guid_type.into(),       // guid
            str_type.into(),        // name
            type_group_type.into(), // group
        ],
        false,
    );

    // Construct the `MunFunctionSignature` type
    let function_signature_type = context.opaque_struct_type("struct.MunFunctionSignature");
    function_signature_type.set_body(
        &[
            str_type.into(),                                     // name
            type_info_type.ptr_type(AddressSpace::Const).into(), // arg_types
            type_info_type.ptr_type(AddressSpace::Const).into(), // return_type
            context.i16_type().into(),                           // num_arg_types
            privacy_type.into(),                                 // privacy
        ],
        false,
    );

    // Construct the `MunFunctionInfo` struct
    let function_info_type = context.opaque_struct_type("struct.MunFunctionInfo");
    function_info_type.set_body(
        &[
            function_signature_type.into(), // signature
            context
                .void_type()
                .fn_type(&[], false)
                .ptr_type(AddressSpace::Const)
                .into(), // fn_ptr
        ],
        false,
    );

    // Construct the `MunStructInfo` struct
    let struct_info_type = context.opaque_struct_type("struct.MunStructInfo");
    struct_info_type.set_body(
        &[
            str_type.into(),                                         // name
            str_type.ptr_type(AddressSpace::Const).into(),           // field_names
            type_info_type.ptr_type(AddressSpace::Const).into(),     // field_types
            context.i16_type().ptr_type(AddressSpace::Const).into(), // field_offsets
            context.i16_type().ptr_type(AddressSpace::Const).into(), // field_sizes
            context.i16_type().into(),                               // num_fields
        ],
        false,
    );

    // Construct the `MunModuleInfo` struct
    let module_info_type = context.opaque_struct_type("struct.MunModuleInfo");
    module_info_type.set_body(
        &[
            str_type.into(),                                         // path
            function_info_type.ptr_type(AddressSpace::Const).into(), // functions
            context.i32_type().into(),                               // num_functions
            struct_info_type.ptr_type(AddressSpace::Const).into(),   // structs
            context.i32_type().into(),                               // num_structs
        ],
        false,
    );

    // Construct the `MunDispatchTable` struct
    let dispatch_table_type = context.opaque_struct_type("struct.MunDispatchTable");
    dispatch_table_type.set_body(
        &[
            function_signature_type.ptr_type(AddressSpace::Const).into(), // signatures
            context
                .void_type()
                .fn_type(&[], false)
                .ptr_type(AddressSpace::Generic)
                .ptr_type(AddressSpace::Const)
                .into(), // fn_ptrs
            context.i32_type().into(),                                    // num_entries
        ],
        false,
    );

    // Construct the `MunAssemblyInfo` struct
    let assembly_info_type = context.opaque_struct_type("struct.MunAssemblyInfo");
    assembly_info_type.set_body(
        &[
            module_info_type.into(),
            dispatch_table_type.into(),
            str_type.ptr_type(AddressSpace::Const).into(),
            context.i32_type().into(),
        ],
        false,
    );

    AbiTypes {
        guid_type,
        type_group_type,
        privacy_type,
        type_info_type,
        function_signature_type,
        function_info_type,
        struct_info_type,
        module_info_type,
        dispatch_table_type,
        assembly_info_type,
    }
}

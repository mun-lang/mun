use crate::code_gen::{gen_global, gen_struct_ptr_array, intern_string};
use crate::ir::{
    abi_types::{gen_abi_types, AbiTypes},
    dispatch_table::{DispatchTable, DispatchableFunction},
    function,
    type_table::TypeTable,
};
use crate::type_info::TypeInfo;
use crate::value::IrValueContext;
use crate::IrDatabase;
use hir::Ty;
use inkwell::{
    attributes::Attribute,
    module::Linkage,
    values::{GlobalValue, PointerValue, StructValue},
    AddressSpace,
};
use std::collections::HashSet;

/// Construct a `MunFunctionPrototype` struct for the specified HIR function.
fn gen_prototype_from_function<D: IrDatabase>(
    db: &D,
    context: &IrValueContext,
    types: &AbiTypes,
    function: hir::Function,
) -> StructValue {
    let module = context.module;
    let name = function.name(db).to_string();

    let name_ir = intern_string(context, &name, &name);
    let _visibility = match function.visibility(db) {
        hir::Visibility::Public => 0,
        _ => 1,
    };

    let fn_sig = function.ty(db).callable_sig(db).unwrap();
    let ret_type_ir = gen_signature_return_type(db, context, types, fn_sig.ret().clone());

    let param_types: Vec<PointerValue> = fn_sig
        .params()
        .iter()
        .map(|ty| {
            TypeTable::get(module, &db.type_info(ty.clone()))
                .unwrap()
                .as_pointer_value()
        })
        .collect();

    let param_types = gen_struct_ptr_array(
        context,
        types.type_info_type,
        &param_types,
        &format!("fn_sig::<{}>::arg_types", name),
    );
    let num_params = fn_sig.params().len();

    let signature = types.function_signature_type.const_named_struct(&[
        param_types.into(),
        ret_type_ir.into(),
        module
            .get_context()
            .i16_type()
            .const_int(num_params as u64, false)
            .into(),
    ]);

    types
        .function_prototype_type
        .const_named_struct(&[name_ir.into(), signature.into()])
}

/// Construct a `MunFunctionPrototype` struct for the specified dispatch table function.
fn gen_prototype_from_dispatch_entry(
    context: &IrValueContext,
    types: &AbiTypes,
    function: &DispatchableFunction,
) -> StructValue {
    let module = context.module;
    let name_str = intern_string(
        &context,
        &function.prototype.name,
        &format!("fn_sig::<{}>::name", function.prototype.name),
    );
    //    let _visibility = match function.visibility(db) {
    //        hir::Visibility::Public => 0,
    //        _ => 1,
    //    };
    let ret_type_ir = gen_signature_return_type_from_type_info(
        context,
        types,
        function.prototype.ret_type.clone(),
    );
    let param_types: Vec<PointerValue> = function
        .prototype
        .arg_types
        .iter()
        .map(|type_info| {
            TypeTable::get(module, type_info)
                .unwrap()
                .as_pointer_value()
        })
        .collect();
    let param_types = gen_struct_ptr_array(
        context,
        types.type_info_type,
        &param_types,
        &format!("{}_param_types", function.prototype.name),
    );
    let num_params = function.prototype.arg_types.len();

    let signature = types.function_signature_type.const_named_struct(&[
        param_types.into(),
        ret_type_ir.into(),
        module
            .get_context()
            .i16_type()
            .const_int(num_params as u64, false)
            .into(),
    ]);

    types
        .function_prototype_type
        .const_named_struct(&[name_str.into(), signature.into()])
}

/// Given a function, construct a pointer to a `MunTypeInfo` global that represents the return type
/// of the function; or `null` if the return type is empty.
fn gen_signature_return_type<D: IrDatabase>(
    db: &D,
    context: &IrValueContext,
    types: &AbiTypes,
    ret_type: Ty,
) -> PointerValue {
    gen_signature_return_type_from_type_info(
        context,
        types,
        if ret_type.is_empty() {
            None
        } else {
            Some(db.type_info(ret_type))
        },
    )
}

/// Given a function, construct a pointer to a `MunTypeInfo` global that represents the return type
/// of the function; or `null` if the return type is empty.
fn gen_signature_return_type_from_type_info(
    context: &IrValueContext,
    types: &AbiTypes,
    ret_type: Option<TypeInfo>,
) -> PointerValue {
    if let Some(ret_type) = ret_type {
        TypeTable::get(context.module, &ret_type)
            .unwrap()
            .as_pointer_value()
    } else {
        types
            .type_info_type
            .ptr_type(AddressSpace::Generic)
            .const_null()
    }
}

/// Construct a global that holds a reference to all functions. e.g.:
/// MunFunctionDefinition[] definitions = { ... }
fn get_function_definition_array<'a, D: IrDatabase>(
    db: &D,
    context: &IrValueContext,
    types: &AbiTypes,
    functions: impl Iterator<Item = &'a hir::Function>,
) -> GlobalValue {
    let module = context.module;
    let function_infos: Vec<StructValue> = functions
        .map(|f| {
            let name = f.name(db).to_string();
            // Get the function from the cloned module and modify the linkage of the function.
            let value = module
                // If a wrapper function exists, use that (required for struct types)
                .get_function(&format!("{}_wrapper", name))
                // Otherwise, use the normal function
                .or_else(|| module.get_function(&name))
                .unwrap();
            value.set_linkage(Linkage::Private);

            // Generate the signature from the function
            let prototype = gen_prototype_from_function(db, context, types, *f);

            // Generate the function info value
            types.function_definition_type.const_named_struct(&[
                prototype.into(),
                value.as_global_value().as_pointer_value().into(),
            ])
        })
        .collect();
    let function_infos = types.function_definition_type.const_array(&function_infos);
    gen_global(module, &function_infos, "fn.get_info.functions")
}

/// Generate the dispatch table information. e.g.:
/// ```c
/// MunDispatchTable dispatchTable = { ... }
/// ```
fn gen_dispatch_table(
    context: &IrValueContext,
    types: &AbiTypes,
    dispatch_table: &DispatchTable,
) -> StructValue {
    let module = context.module;
    // Generate a vector with all the function signatures
    let signatures: Vec<StructValue> = dispatch_table
        .entries()
        .iter()
        .map(|entry| gen_prototype_from_dispatch_entry(context, types, entry))
        .collect();

    // Construct an IR array from the signatures
    let signatures = gen_global(
        module,
        &types.function_signature_type.const_array(&signatures),
        "fn.get_info.dispatchTable.signatures",
    );

    // Get the pointer to the global table (or nullptr if no global table was defined).
    let dispatch_table_ptr = dispatch_table
        .global_value()
        .map(|_g|
            // TODO: This is a hack, the passed module here is a clone of the module with which the
            // dispatch table was created. Because of this we have to lookup the dispatch table
            // global again. There is however not a `GlobalValue::get_name` method so I just
            // hardcoded the name here.
            module.get_global("dispatchTable").unwrap().as_pointer_value())
        .unwrap_or_else(|| {
            module
                .get_context()
                .void_type()
                .fn_type(&[], false)
                .ptr_type(AddressSpace::Generic)
                .ptr_type(AddressSpace::Generic)
                .const_null()
        });

    types.dispatch_table_type.const_named_struct(&[
        signatures.as_pointer_value().into(),
        dispatch_table_ptr.into(),
        module
            .get_context()
            .i32_type()
            .const_int(dispatch_table.entries().len() as u64, false)
            .into(),
    ])
}

/// Constructs IR that exposes the types and symbols in the specified module. A function called
/// `get_info` is constructed that returns a struct `MunAssemblyInfo`. See the `mun_abi` crate
/// for the ABI that `get_info` exposes.
pub(super) fn gen_reflection_ir(
    db: &impl IrDatabase,
    context: &IrValueContext,
    api: &HashSet<hir::Function>,
    dispatch_table: &DispatchTable,
    type_table: &TypeTable,
) {
    let module = context.module;

    // Get all the types
    let abi_types = gen_abi_types(context.type_context);

    let num_functions = api.len();
    let function_info = get_function_definition_array(db, context, &abi_types, api.iter());

    let type_table_ir = if let Some(type_table) = module.get_global(TypeTable::NAME) {
        type_table.as_pointer_value()
    } else {
        type_table.ty().ptr_type(AddressSpace::Generic).const_null()
    };

    // Construct the module info struct
    let module_info = abi_types.module_info_type.const_named_struct(&[
        intern_string(context, "", "module_info::path").into(),
        function_info.as_pointer_value().into(),
        module
            .get_context()
            .i32_type()
            .const_int(num_functions as u64, false)
            .into(),
        type_table_ir.into(),
        module
            .get_context()
            .i32_type()
            .const_int(type_table.num_types() as u64, false)
            .into(),
    ]);

    // Construct the dispatch table struct
    let dispatch_table = gen_dispatch_table(context, &abi_types, dispatch_table);

    // Construct the actual `get_info` function
    gen_get_info_fn(db, context, &abi_types, module_info, dispatch_table);
    gen_set_allocator_handle_fn(db, context);
}

/// Construct the actual `get_info` function.
fn gen_get_info_fn(
    db: &impl IrDatabase,
    context: &IrValueContext,
    abi_types: &AbiTypes,
    module_info: StructValue,
    dispatch_table: StructValue,
) {
    let target = db.target();
    let str_type = context.context.i8_type().ptr_type(AddressSpace::Generic);

    // Construct the return type of the `get_info` method. Depending on the C ABI this is either the
    // `MunAssemblyInfo` struct or void. On windows the return argument is passed back to the caller
    // through a pointer to the return type as the first argument. e.g.:
    // On Windows:
    // ```c
    // void get_info(MunModuleInfo* result) {...}
    // ```
    // Whereas on other platforms the signature of the `get_info` function is:
    // ```c
    // MunModuleInfo get_info() { ... }
    // ```
    let get_symbols_type = if target.options.is_like_windows {
        context.context.void_type().fn_type(
            &[abi_types
                .assembly_info_type
                .ptr_type(AddressSpace::Generic)
                .into()],
            false,
        )
    } else {
        abi_types.assembly_info_type.fn_type(&[], false)
    };

    let get_symbols_fn =
        context
            .module
            .add_function("get_info", get_symbols_type, Some(Linkage::DLLExport));

    if target.options.is_like_windows {
        get_symbols_fn.add_attribute(
            inkwell::attributes::AttributeLoc::Param(0),
            context
                .context
                .create_enum_attribute(Attribute::get_named_enum_kind_id("sret"), 1),
        );
    }

    let builder = db.context().create_builder();
    let body_ir = db.context().append_basic_block(&get_symbols_fn, "body");
    builder.position_at_end(&body_ir);

    // Get a pointer to the IR value that will hold the return value. Again this differs depending
    // on the C ABI.
    let result_ptr = if target.options.is_like_windows {
        get_symbols_fn
            .get_nth_param(0)
            .unwrap()
            .into_pointer_value()
    } else {
        builder.build_alloca(abi_types.assembly_info_type, "")
    };

    // Get access to the structs internals
    let symbols_addr = unsafe { builder.build_struct_gep(result_ptr, 0, "symbols") };
    let dispatch_table_addr = unsafe { builder.build_struct_gep(result_ptr, 1, "dispatch_table") };
    let dependencies_addr = unsafe { builder.build_struct_gep(result_ptr, 2, "dependencies") };
    let num_dependencies_addr =
        unsafe { builder.build_struct_gep(result_ptr, 3, "num_dependencies") };

    // Assign the struct values one by one.
    builder.build_store(symbols_addr, module_info);
    builder.build_store(dispatch_table_addr, dispatch_table);
    builder.build_store(
        dependencies_addr,
        str_type.ptr_type(AddressSpace::Generic).const_null(),
    );
    builder.build_store(
        num_dependencies_addr,
        context.context.i32_type().const_int(0 as u64, false),
    );

    // Construct the return statement of the function.
    if target.options.is_like_windows {
        builder.build_return(None);
    } else {
        builder.build_return(Some(&builder.build_load(result_ptr, "")));
    }

    // Run the function optimizer on the generate function
    function::create_pass_manager(&context.module, db.optimization_lvl()).run_on(&get_symbols_fn);
}

fn gen_set_allocator_handle_fn(db: &impl IrDatabase, context: &IrValueContext) {
    let allocator_handle_type = context.context.i8_type().ptr_type(AddressSpace::Generic);

    let set_allocator_handle_fn_type = context
        .context
        .void_type()
        .fn_type(&[allocator_handle_type.into()], false);

    let set_allocator_handle_fn = context.module.add_function(
        "set_allocator_handle",
        set_allocator_handle_fn_type,
        Some(Linkage::DLLExport),
    );

    let builder = db.context().create_builder();
    let body_ir = db
        .context()
        .append_basic_block(&set_allocator_handle_fn, "body");
    builder.position_at_end(&body_ir);

    if let Some(allocator_handle_global) = context.module.get_global("allocatorHandle") {
        builder.build_store(
            allocator_handle_global.as_pointer_value(),
            set_allocator_handle_fn.get_nth_param(0).unwrap(),
        );
    }

    builder.build_return(None);
}

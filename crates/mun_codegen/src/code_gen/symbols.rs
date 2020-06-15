use crate::ir::types as ir;
use crate::ir::{
    dispatch_table::{DispatchTable, DispatchableFunction},
    function,
    type_table::TypeTable,
};
use crate::type_info::TypeInfo;
use crate::value::{AsValue, CanInternalize, Global, IrValueContext, IterAsIrValue, Value};
use crate::IrDatabase;
use hir::Ty;
use inkwell::{attributes::Attribute, module::Linkage, AddressSpace};
use std::collections::HashSet;
use std::ffi::CString;

/// Construct a `MunFunctionPrototype` struct for the specified HIR function.
fn gen_prototype_from_function<D: IrDatabase>(
    db: &D,
    context: &IrValueContext,
    function: hir::Function,
) -> ir::FunctionPrototype {
    let module = context.module;
    let name = function.name(db).to_string();

    // Internalize the name of the function prototype
    let name_str = CString::new(name.clone())
        .expect("function prototype name is not a valid CString")
        .intern(format!("fn_sig::<{}>::name", &name), context);

    // Get the `ir::TypeInfo` pointer for the return type of the function
    let fn_sig = function.ty(db).callable_sig(db).unwrap();
    let return_type = gen_signature_return_type(db, context, fn_sig.ret().clone());

    // Construct an array of pointers to `ir::TypeInfo`s for the arguments of the prototype
    let arg_types = fn_sig
        .params()
        .iter()
        .map(|ty| {
            TypeTable::get(module, &db.type_info(ty.clone()))
                .expect("expected a TypeInfo for a prototype argument but it was not found")
                .as_value(context)
        })
        .into_const_private_pointer_or_null(format!("fn_sig::<{}>::arg_types", &name), context);

    ir::FunctionPrototype {
        name: name_str.as_value(context),
        signature: ir::FunctionSignature {
            arg_types,
            return_type,
            num_arg_types: fn_sig.params().len() as u16,
        },
    }
}

/// Construct a `MunFunctionPrototype` struct for the specified dispatch table function.
fn gen_prototype_from_dispatch_entry(
    context: &IrValueContext,
    function: &DispatchableFunction,
) -> ir::FunctionPrototype {
    let module = context.module;

    // Internalize the name of the function prototype
    let name_str = CString::new(function.prototype.name.clone())
        .expect("function prototype name is not a valid CString")
        .intern(
            format!("fn_sig::<{}>::name", function.prototype.name),
            context,
        );

    // Get the `ir::TypeInfo` pointer for the return type of the function
    let return_type =
        gen_signature_return_type_from_type_info(context, function.prototype.ret_type.clone());

    // Construct an array of pointers to `ir::TypeInfo`s for the arguments of the prototype
    let arg_types = function
        .prototype
        .arg_types
        .iter()
        .map(|type_info| {
            TypeTable::get(module, type_info)
                .expect("expected a TypeInfo for a prototype argument but it was not found")
                .as_value(context)
        })
        .into_const_private_pointer_or_null(
            format!("{}_param_types", function.prototype.name),
            context,
        );

    ir::FunctionPrototype {
        name: name_str.as_value(context),
        signature: ir::FunctionSignature {
            arg_types,
            return_type,
            num_arg_types: function.prototype.arg_types.len() as u16,
        },
    }
}

/// Given a function, construct a pointer to a `ir::TypeInfo` global that represents the return type
/// of the function; or `null` if the return type is empty.
fn gen_signature_return_type<D: IrDatabase>(
    db: &D,
    context: &IrValueContext,
    ret_type: Ty,
) -> Value<*const ir::TypeInfo> {
    gen_signature_return_type_from_type_info(
        context,
        if ret_type.is_empty() {
            None
        } else {
            Some(db.type_info(ret_type))
        },
    )
}

/// Given a function, construct a pointer to a `ir::TypeInfo` global that represents the return type
/// of the function; or `null` if the return type is empty.
fn gen_signature_return_type_from_type_info(
    context: &IrValueContext,
    ret_type: Option<TypeInfo>,
) -> Value<*const ir::TypeInfo> {
    ret_type
        .map(|info| {
            TypeTable::get(context.module, &info)
                .expect("could not find TypeInfo that should definitely be there")
                .as_value(context)
        })
        .unwrap_or_else(|| Value::null(context))
}

/// Construct a global that holds a reference to all functions. e.g.:
/// MunFunctionDefinition[] definitions = { ... }
fn get_function_definition_array<'a, D: IrDatabase>(
    db: &D,
    context: &IrValueContext,
    functions: impl Iterator<Item = &'a hir::Function>,
) -> Global<[ir::FunctionDefinition]> {
    let module = context.module;
    functions
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
            let prototype = gen_prototype_from_function(db, context, *f);
            ir::FunctionDefinition {
                prototype,
                fn_ptr: Value::from_raw(value.as_global_value().as_pointer_value()),
            }
        })
        .as_value(context)
        .into_const_private_global("fn.get_info.functions", context)
}

/// Generate the dispatch table information. e.g.:
/// ```c
/// MunDispatchTable dispatchTable = { ... }
/// ```
fn gen_dispatch_table(
    context: &IrValueContext,
    dispatch_table: &DispatchTable,
) -> ir::DispatchTable {
    let module = context.module;

    // Generate an internal array that holds all the function prototypes
    let prototypes = dispatch_table
        .entries()
        .iter()
        .map(|entry| gen_prototype_from_dispatch_entry(context, entry))
        .into_const_private_pointer("fn.get_info.dispatchTable.signatures", context);

    // Get the pointer to the global table (or nullptr if no global table was defined).
    let fn_ptrs = dispatch_table
        .global_value()
        .map(|_g|
            // TODO: This is a hack, the passed module here is a clone of the module with which the
            // dispatch table was created. Because of this we have to lookup the dispatch table
            // global again. There is however not a `GlobalValue::get_name` method so I just
            // hardcoded the name here.
            Value::from_raw(module.get_global("dispatchTable").unwrap().as_pointer_value()))
        .unwrap_or_else(|| Value::null(context));

    ir::DispatchTable {
        prototypes,
        fn_ptrs,
        num_entries: dispatch_table.entries().len() as u32,
    }
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

    let num_functions = api.len() as u32;
    let functions = get_function_definition_array(db, context, api.iter());

    // Get the TypeTable global
    let types = TypeTable::find_global(module)
        .map(|g| g.as_value(context))
        .unwrap_or_else(|| Value::null(context));

    // Construct the module info struct
    let module_info = ir::ModuleInfo {
        path: CString::new("")
            .unwrap()
            .intern("module_info::path", context)
            .as_value(context),
        functions: functions.as_value(context),
        num_functions,
        types,
        num_types: type_table.num_types() as u32,
    };

    // Construct the dispatch table struct
    let dispatch_table = gen_dispatch_table(context, dispatch_table);

    // Construct the actual `get_info` function
    gen_get_info_fn(db, context, module_info, dispatch_table);
    gen_set_allocator_handle_fn(db, context);
    gen_get_version_fn(db, context);
}

/// Construct the actual `get_info` function.
fn gen_get_info_fn(
    db: &impl IrDatabase,
    context: &IrValueContext,
    module_info: ir::ModuleInfo,
    dispatch_table: ir::DispatchTable,
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
        Value::<fn(*mut ir::AssemblyInfo)>::get_ir_type(context.type_context)
    } else {
        Value::<fn() -> ir::AssemblyInfo>::get_ir_type(context.type_context)
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
        builder.build_alloca(
            Value::<ir::AssemblyInfo>::get_ir_type(context.type_context),
            "",
        )
    };

    // Get access to the structs internals
    let symbols_addr = unsafe { builder.build_struct_gep(result_ptr, 0, "symbols") };
    let dispatch_table_addr = unsafe { builder.build_struct_gep(result_ptr, 1, "dispatch_table") };
    let dependencies_addr = unsafe { builder.build_struct_gep(result_ptr, 2, "dependencies") };
    let num_dependencies_addr =
        unsafe { builder.build_struct_gep(result_ptr, 3, "num_dependencies") };

    // Assign the struct values one by one.
    builder.build_store(symbols_addr, module_info.as_value(context).value);
    builder.build_store(dispatch_table_addr, dispatch_table.as_value(context).value);
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

/// Generates a method `void set_allocator_handle(void*)` that stores the argument into the global
/// `allocatorHandle`. This global is used internally to reference the allocator used by this
/// munlib.
fn gen_set_allocator_handle_fn(db: &impl IrDatabase, context: &IrValueContext) {
    let set_allocator_handle_fn = context.module.add_function(
        "set_allocator_handle",
        Value::<fn(*const u8)>::get_ir_type(context.type_context),
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

/// Generates a `get_version` method that returns the current abi version.
/// Specifically, it returns the abi version the function was generated in.
fn gen_get_version_fn(db: &impl IrDatabase, context: &IrValueContext) {
    let get_version_fn = context.module.add_function(
        abi::GET_VERSION_FN_NAME,
        Value::<fn() -> u32>::get_ir_type(context.type_context),
        Some(Linkage::DLLExport),
    );

    let builder = db.context().create_builder();
    let body_ir = db.context().append_basic_block(&get_version_fn, "body");
    builder.position_at_end(&body_ir);

    builder.build_return(Some(&abi::ABI_VERSION.as_value(context).value));
}

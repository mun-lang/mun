use std::convert::TryFrom;
use std::{collections::HashSet, ffi::CString};

use inkwell::{attributes::Attribute, module::Linkage, types::AnyType};

use hir::{HirDatabase, TyKind};
use ir_type_builder::TypeIdBuilder;

use crate::{
    ir::ty::HirTypeCache,
    ir::types as ir,
    ir::{
        dispatch_table::{DispatchTable, DispatchableFunction},
        function,
        type_table::TypeTable,
    },
    value::{
        AsValue, CanInternalize, Global, IrValueContext, IterAsIrValue, SizedValueType, Value,
    },
};
use crate::type_info::HasStaticTypeId;

mod ir_type_builder;

/// Construct a `MunFunctionPrototype` struct for the specified HIR function.
fn gen_prototype_from_function<'ink>(
    db: &dyn HirDatabase,
    context: &IrValueContext<'ink, '_, '_>,
    function: hir::Function,
    hir_types: &HirTypeCache,
    ir_type_builder: &TypeIdBuilder<'ink, '_, '_, '_>,
) -> ir::FunctionPrototype<'ink> {
    let name = function.full_name(db);

    // Internalize the name of the function prototype
    let name_str = CString::new(name.clone())
        .expect("function prototype name is not a valid CString")
        .intern(format!("fn_sig::<{}>::name", &name), context);

    // Get the `ir::TypeInfo` pointer for the return type of the function
    let fn_sig = function.ty(db).callable_sig(db).unwrap();
    let return_type = if fn_sig.ret().is_empty() {
        ir_type_builder.construct_from_type_id(<() as HasStaticTypeId>::type_id())
    } else {
        ir_type_builder.construct_from_type_id(&hir_types.type_id(&fn_sig.ret()))
    };

    // Construct an array of pointers to `ir::TypeInfo`s for the arguments of the prototype
    let arg_types = fn_sig
        .params()
        .iter()
        .map(|ty| ir_type_builder.construct_from_type_id(&hir_types.type_id(ty)))
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
fn gen_prototype_from_dispatch_entry<'ink>(
    context: &IrValueContext<'ink, '_, '_>,
    function: &DispatchableFunction,
    ir_type_builder: &TypeIdBuilder<'ink, '_, '_, '_>,
) -> ir::FunctionPrototype<'ink> {
    // Internalize the name of the function prototype
    let name_str = CString::new(function.prototype.name.clone())
        .expect("function prototype name is not a valid CString")
        .intern(
            format!("fn_sig::<{}>::name", function.prototype.name),
            context,
        );

    // Get the `ir::TypeInfo` pointer for the return type of the function
    let return_type = ir_type_builder.construct_from_type_id(&function.prototype.ret_type);

    // Construct an array of pointers to `ir::TypeInfo`s for the arguments of the prototype
    let arg_types = function
        .prototype
        .arg_types
        .iter()
        .map(|type_info| ir_type_builder.construct_from_type_id(type_info))
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

/// Construct a global that holds a reference to all types. e.g.:
/// MunTypeInfo[] definitions = { ... }
fn get_type_definition_array<'ink, 'a>(
    db: &dyn HirDatabase,
    context: &IrValueContext<'ink, '_, '_>,
    types: impl Iterator<Item = hir::Ty>,
    hir_types: &HirTypeCache,
    ir_type_builder: &TypeIdBuilder<'ink, '_, '_, '_>,
) -> Value<'ink, *const ir::TypeInfo<'ink>> {
    types
        .map(|type_info| match type_info.interned() {
            TyKind::Struct(s) => {
                let inkwell_type = hir_types.get_struct_type(*s);
                let struct_name = s.full_name(db);
                ir::TypeInfo {
                    name: CString::new(struct_name.clone())
                        .expect("typename is not a valid CString")
                        .intern(format!("type_info::<{}>::name", struct_name), context)
                        .as_value(context),
                    size_in_bits: context
                        .type_context
                        .target_data
                        .get_bit_size(&inkwell_type)
                        .try_into()
                        .expect("could not convert size in bits to smaller size"),
                    alignment: context
                        .type_context
                        .target_data
                        .get_abi_alignment(&inkwell_type)
                        .try_into()
                        .expect("could not convert alignment to smaller size"),
                    data: ir::TypeInfoData::Struct(gen_struct_info(
                        db,
                        *s,
                        context,
                        hir_types,
                        ir_type_builder,
                    )),
                }

            }
            _ => unreachable!("unsupported export type"),
        })
        .into_const_private_pointer_or_null("fn.get_info.types", context)
}

fn gen_struct_info<'ink>(
    db: &dyn HirDatabase,
    hir_struct: hir::Struct,
    context: &IrValueContext<'ink, '_, '_>,
    hir_types: &HirTypeCache,
    ir_type_builder: &TypeIdBuilder<'ink, '_, '_, '_>,
) -> ir::StructInfo<'ink> {
    let struct_ir = hir_types.get_struct_type(hir_struct);
    let name = hir_struct.full_name(db);
    let fields = hir_struct.fields(db);

    // Construct an array of field names (or null if there are no fields)
    let field_names = fields
        .iter()
        .enumerate()
        .map(|(idx, field)| {
            CString::new(field.name(db).to_string())
                .expect("field name is not a valid CString")
                .intern(
                    format!("struct_info::<{}>::field_names.{}", name, idx),
                    context,
                )
                .as_value(context)
        })
        .into_const_private_pointer_or_null(
            format!("struct_info::<{}>::field_names", name),
            context,
        );

    // Construct an array of field types (or null if there are no fields)
    let field_types = fields
        .iter()
        .map(|field| {
            let field_type_info = hir_types.type_id(&field.ty(db));
            ir_type_builder.construct_from_type_id(&field_type_info)
        })
        .into_const_private_pointer_or_null(
            format!("struct_info::<{}>::field_types", name),
            context,
        );

    // Construct an array of field offsets (or null if there are no fields)
    let field_offsets = fields
        .iter()
        .enumerate()
        .map(|(idx, _)| {
            context
                .type_context
                .target_data
                .offset_of_element(&struct_ir, idx as u32)
                .unwrap() as u16
        })
        .into_const_private_pointer_or_null(
            format!("struct_info::<{}>::field_offsets", name),
            context,
        );

    ir::StructInfo {
        field_names,
        field_types,
        field_offsets,
        num_fields: fields
            .len()
            .try_into()
            .expect("could not convert num_fields to smaller bit size"),
        memory_kind: hir_struct.data(db.upcast()).memory_kind,
    }
}

/// Construct a global that holds a reference to all functions. e.g.:
/// MunFunctionDefinition[] definitions = { ... }
fn get_function_definition_array<'ink, 'a>(
    db: &dyn HirDatabase,
    context: &IrValueContext<'ink, '_, '_>,
    functions: impl Iterator<Item = &'a hir::Function>,
    hir_types: &HirTypeCache,
    ir_type_builder: &TypeIdBuilder<'ink, '_, '_, '_>,
) -> Global<'ink, [ir::FunctionDefinition<'ink>]> {
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
            let prototype = gen_prototype_from_function(db, context, *f, hir_types, ir_type_builder);
            ir::FunctionDefinition {
                prototype,
                fn_ptr: Value::<*const fn()>::with_cast(
                    value.as_global_value().as_pointer_value(),
                    context,
                ),
            }
        })
        .into_value(context)
        .into_const_private_global("fn.get_info.functions", context)
}

/// Generate the type lookup table information. e.g.:
/// ```c
/// MunTypeLut typeLut = { ... }
/// ```
fn gen_type_lut<'ink>(
    context: &IrValueContext<'ink, '_, '_>,
    type_table: &TypeTable,
    ir_type_builder: &TypeIdBuilder<'ink, '_, '_, '_>,
) -> ir::TypeLut<'ink> {
    let module = context.module;

    // Get a list of all Guids
    let type_ids = type_table
        .entries()
        .iter()
        .map(|ty| ir_type_builder.construct_from_type_id(ty))
        .into_const_private_pointer("fn.get_info.typeLut.typeIds", context);

    let type_names = type_table
        .entries()
        .iter()
        .map(|ty| {
            CString::new(ty.name.as_str())
                .expect("unable to create CString from typeinfo name")
                .intern(&ty.name, context)
                .as_value(context)
        })
        .into_const_private_pointer("fn.get_info.typeLut.typeNames", context);

    let type_ptrs = TypeTable::find_global(module)
        .map(|type_table| {
            Value::<*mut *const std::ffi::c_void>::with_cast(
                type_table.as_value(context).value,
                context,
            )
        })
        .unwrap_or_else(|| Value::null(context));

    ir::TypeLut {
        type_ids,
        type_ptrs,
        type_names,
        num_entries: type_table.num_types().try_into().expect("too many types"),
    }
}

/// Generate the dispatch table information. e.g.:
/// ```c
/// MunDispatchTable dispatchTable = { ... }
/// ```
fn gen_dispatch_table<'ink>(
    context: &IrValueContext<'ink, '_, '_>,
    dispatch_table: &DispatchTable<'ink>,
    ir_type_builder: &TypeIdBuilder<'ink, '_, '_, '_>,
) -> ir::DispatchTable<'ink> {
    let module = context.module;

    // Generate an internal array that holds all the function prototypes
    let prototypes = dispatch_table
        .entries()
        .iter()
        .map(|entry| gen_prototype_from_dispatch_entry(context, entry, ir_type_builder))
        .into_const_private_pointer("fn.get_info.dispatchTable.signatures", context);

    // Get the pointer to the global table (or nullptr if no global table was defined).
    let fn_ptrs = dispatch_table
        .global_value()
        .map(|_g|
            // TODO: This is a hack, the passed module here is a clone of the module with which the
            // dispatch table was created. Because of this we have to lookup the dispatch table
            // global again. There is however not a `GlobalValue::get_name` method so I just
            // hardcoded the name here.
            Value::<*mut *const fn()>::with_cast(module.get_global("dispatchTable").unwrap().as_pointer_value(), context))
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
#[allow(clippy::too_many_arguments)]
pub(super) fn gen_reflection_ir<'db, 'ink>(
    db: &'db dyn HirDatabase,
    context: &IrValueContext<'ink, '_, '_>,
    function_definitions: &HashSet<hir::Function>,
    type_definitions: &HashSet<hir::Ty>,
    dispatch_table: &DispatchTable<'ink>,
    type_table: &TypeTable<'ink>,
    hir_types: &HirTypeCache<'db, 'ink>,
    optimization_level: inkwell::OptimizationLevel,
    dependencies: Vec<String>,
) {
    let ir_type_builder = TypeIdBuilder::new(context);

    let num_functions = function_definitions.len() as u32;
    let functions =
        get_function_definition_array(db, context, function_definitions.iter(), hir_types, &ir_type_builder);

    // Get the TypeTable global
    let num_types = type_definitions.len() as u32;
    let types = get_type_definition_array(db, context, type_definitions.iter().cloned(), hir_types, &ir_type_builder);

    // Construct the module info struct
    let module_info = ir::ModuleInfo {
        path: CString::new("")
            .unwrap()
            .intern("module_info::path", context)
            .as_value(context),
        functions: functions.as_value(context),
        num_functions,
        types,
        num_types,
    };

    // Construct the dispatch table struct
    let dispatch_table = gen_dispatch_table(context, dispatch_table, &ir_type_builder);

    let type_lut = gen_type_lut(context, type_table, &ir_type_builder);

    // Construct the actual `get_info` function
    gen_get_info_fn(
        db,
        context,
        module_info,
        dispatch_table,
        type_lut,
        optimization_level,
        dependencies,
    );
    gen_set_allocator_handle_fn(context);
    gen_get_version_fn(context);
}

/// Construct the actual `get_info` function.
fn gen_get_info_fn<'ink>(
    db: &dyn HirDatabase,
    context: &IrValueContext<'ink, '_, '_>,
    module_info: ir::ModuleInfo<'ink>,
    dispatch_table: ir::DispatchTable<'ink>,
    type_lut: ir::TypeLut<'ink>,
    optimization_level: inkwell::OptimizationLevel,
    dependencies: Vec<String>,
) {
    let target = db.target();

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
        Value::<'ink, fn(*mut ir::AssemblyInfo<'ink>)>::get_ir_type(context.type_context)
    } else {
        Value::<'ink, fn() -> ir::AssemblyInfo<'ink>>::get_ir_type(context.type_context)
    };

    let get_symbols_fn =
        context
            .module
            .add_function("get_info", get_symbols_type, Some(Linkage::DLLExport));

    if target.options.is_like_windows {
        let type_attribute = context.context.create_type_attribute(
            Attribute::get_named_enum_kind_id("sret"),
            ir::AssemblyInfo::get_ir_type(context.type_context).as_any_type_enum(),
        );

        get_symbols_fn.add_attribute(inkwell::attributes::AttributeLoc::Param(0), type_attribute);
    }

    let builder = context.context.create_builder();
    let body_ir = context.context.append_basic_block(get_symbols_fn, "body");
    builder.position_at_end(body_ir);

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
    let symbols_addr = builder
        .build_struct_gep(result_ptr, 1, "symbols")
        .expect("could not retrieve `symbols` from result struct");
    let dispatch_table_addr = builder
        .build_struct_gep(result_ptr, 3, "dispatch_table")
        .expect("could not retrieve `dispatch_table` from result struct");
    let type_lut_addr = builder
        .build_struct_gep(result_ptr, 5, "type_lut")
        .expect("could not retrieve `type_lut` from result struct");
    let dependencies_addr = builder
        .build_struct_gep(result_ptr, 7, "dependencies")
        .expect("could not retrieve `dependencies` from result struct");
    let num_dependencies_addr = builder
        .build_struct_gep(result_ptr, 9, "num_dependencies")
        .expect("could not retrieve `num_dependencies` from result struct");

    // Assign the struct values one by one.
    builder.build_store(symbols_addr, module_info.as_value(context).value);
    builder.build_store(dispatch_table_addr, dispatch_table.as_value(context).value);
    builder.build_store(type_lut_addr, type_lut.as_value(context).value);
    builder.build_store(
        dependencies_addr,
        dependencies
            .iter()
            .enumerate()
            .map(|(idx, name)| {
                CString::new(name.as_str())
                    .expect("could not convert dependency name to string")
                    .intern(format!("dependency{}", idx), context)
                    .as_value(context)
            })
            .into_const_private_pointer_or_null("dependencies", context)
            .value,
    );
    builder.build_store(
        num_dependencies_addr,
        context.context.i32_type().const_int(
            u32::try_from(dependencies.len()).expect("too many dependencies") as u64,
            false,
        ),
    );

    // Construct the return statement of the function.
    if target.options.is_like_windows {
        builder.build_return(None);
    } else {
        builder.build_return(Some(&builder.build_load(result_ptr, "")));
    }

    // Run the function optimizer on the generate function
    function::create_pass_manager(context.module, optimization_level).run_on(&get_symbols_fn);
}

/// Generates a method `void set_allocator_handle(void*)` that stores the argument into the global
/// `allocatorHandle`. This global is used internally to reference the allocator used by this
/// munlib.
fn gen_set_allocator_handle_fn(context: &IrValueContext) {
    let set_allocator_handle_fn = context.module.add_function(
        "set_allocator_handle",
        Value::<fn(*const u8)>::get_ir_type(context.type_context),
        Some(Linkage::DLLExport),
    );

    let builder = context.context.create_builder();
    let body_ir = context
        .context
        .append_basic_block(set_allocator_handle_fn, "body");
    builder.position_at_end(body_ir);

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
fn gen_get_version_fn(context: &IrValueContext) {
    let get_version_fn = context.module.add_function(
        abi::GET_VERSION_FN_NAME,
        Value::<fn() -> u32>::get_ir_type(context.type_context),
        Some(Linkage::DLLExport),
    );

    let builder = context.context.create_builder();
    let body_ir = context.context.append_basic_block(get_version_fn, "body");
    builder.position_at_end(body_ir);

    builder.build_return(Some(&abi::ABI_VERSION.as_value(context).value));
}

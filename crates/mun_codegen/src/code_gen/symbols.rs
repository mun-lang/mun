use super::abi_types::{gen_abi_types, AbiTypes};
use crate::ir::{
    dispatch_table::{DispatchTable, DispatchableFunction},
    function,
};
use crate::type_info::{TypeGroup, TypeInfo};
use crate::values::{BasicValue, GlobalValue};
use crate::IrDatabase;
use hir::Ty;
use inkwell::{
    attributes::Attribute,
    module::{Linkage, Module},
    targets::TargetMachine,
    values::{FunctionValue, IntValue, PointerValue, StructValue, UnnamedAddress},
    AddressSpace,
};
use std::collections::{HashMap, HashSet};

struct GlobalArrayValue(GlobalValue, usize);

/// Construct an IR `MunTypeInfo` struct value for the specified `TypeInfo`
fn type_info_ir<D: IrDatabase>(
    db: &D,
    target: &TargetMachine,
    module: &Module,
    types: &AbiTypes,
    global_type_info_lookup_table: &mut HashMap<TypeInfo, PointerValue>,
    type_info: TypeInfo,
) -> PointerValue {
    if let Some(value) = global_type_info_lookup_table.get(&type_info) {
        *value
    } else {
        let context = module.get_context();
        let guid_values: [IntValue; 16] = array_init::array_init(|i| {
            context
                .i8_type()
                .const_int(u64::from(type_info.guid.b[i]), false)
        });

        let type_info_ir = context.const_struct(
            &[
                context.i8_type().const_array(&guid_values).into(),
                intern_string(module, &type_info.name).into(),
                context
                    .i8_type()
                    .const_int(type_info.group.clone().into(), false)
                    .into(),
            ],
            false,
        );

        let type_info_ir = match type_info.group {
            TypeGroup::FundamentalTypes => type_info_ir,
            TypeGroup::StructTypes(s) => {
                let struct_info_ir =
                    gen_struct_info(db, target, module, types, global_type_info_lookup_table, s);
                context.const_struct(&[type_info_ir.into(), struct_info_ir.into()], false)
            }
        };

        let type_info_ir = gen_global(module, &type_info_ir, "").as_pointer_value();
        global_type_info_lookup_table.insert(type_info, type_info_ir);
        type_info_ir
    }
}

/// Intern a string by constructing a global value. Looks something like this:
/// ```c
/// const char[] GLOBAL_ = "str";
/// ```
fn intern_string(module: &Module, str: &str) -> PointerValue {
    let value = module.get_context().const_string(str, true);
    gen_global(module, &value, ".str").as_pointer_value()
}

/// Construct a `MunFunctionSignature` struct for the specified HIR function.
fn gen_signature_from_function<D: IrDatabase>(
    db: &D,
    module: &Module,
    types: &AbiTypes,
    global_type_info_lookup_table: &HashMap<TypeInfo, PointerValue>,
    function: hir::Function,
) -> StructValue {
    let name_str = intern_string(&module, &function.name(db).to_string());
    let _visibility = match function.visibility(db) {
        hir::Visibility::Public => 0,
        _ => 1,
    };

    let fn_sig = function.ty(db).callable_sig(db).unwrap();
    let ret_type_ir = gen_signature_return_type(
        db,
        types,
        global_type_info_lookup_table,
        fn_sig.ret().clone(),
    );

    let params_type_ir = gen_type_info_ptr_array(
        module,
        types,
        global_type_info_lookup_table,
        fn_sig.params().iter().map(|ty| db.type_info(ty.clone())),
    );
    let num_params = fn_sig.params().len();

    types.function_signature_type.const_named_struct(&[
        name_str.into(),
        params_type_ir.into(),
        ret_type_ir.into(),
        module
            .get_context()
            .i16_type()
            .const_int(num_params as u64, false)
            .into(),
        module.get_context().i8_type().const_int(0, false).into(),
    ])
}

/// Construct a `MunFunctionSignature` struct for the specified dispatch table function.
fn gen_signature_from_dispatch_entry<D: IrDatabase>(
    db: &D,
    module: &Module,
    types: &AbiTypes,
    global_type_info_lookup_table: &HashMap<TypeInfo, PointerValue>,
    function: &DispatchableFunction,
) -> StructValue {
    let name_str = intern_string(&module, &function.prototype.name);
    //    let _visibility = match function.visibility(db) {
    //        hir::Visibility::Public => 0,
    //        _ => 1,
    //    };
    let ret_type_ir = gen_signature_return_type_from_type_info(
        db,
        types,
        global_type_info_lookup_table,
        function.prototype.ret_type.clone(),
    );
    let params_type_ir = gen_type_info_ptr_array(
        module,
        types,
        global_type_info_lookup_table,
        function.prototype.arg_types.iter().cloned(),
    );
    let num_params = function.prototype.arg_types.len();

    types.function_signature_type.const_named_struct(&[
        name_str.into(),
        params_type_ir.into(),
        ret_type_ir.into(),
        module
            .get_context()
            .i16_type()
            .const_int(num_params as u64, false)
            .into(),
        module.get_context().i8_type().const_int(0, false).into(),
    ])
}

/// Recursively expands an array of TypeInfo into an array of IR pointers
fn expand_type_info<D: IrDatabase>(
    db: &D,
    target: &TargetMachine,
    module: &Module,
    types: &AbiTypes,
    global_type_info_lookup_table: &mut HashMap<TypeInfo, PointerValue>,
    hir_types: impl Iterator<Item = TypeInfo>,
) -> PointerValue {
    let mut hir_types = hir_types.peekable();
    if hir_types.peek().is_none() {
        types
            .type_info_type
            .ptr_type(AddressSpace::Const)
            .ptr_type(AddressSpace::Const)
            .const_null()
    } else {
        let type_infos = hir_types
            .map(|ty| type_info_ir(db, target, module, types, global_type_info_lookup_table, ty))
            .collect::<Vec<PointerValue>>();

        let type_array_ir = types
            .type_info_type
            .ptr_type(AddressSpace::Const)
            .const_array(&type_infos);

        gen_global(module, &type_array_ir, "").as_pointer_value()
    }
}

/// Generates IR for an array of TypeInfo values
fn gen_type_info_ptr_array(
    module: &Module,
    types: &AbiTypes,
    global_type_info_lookup_table: &HashMap<TypeInfo, PointerValue>,
    hir_types: impl Iterator<Item = TypeInfo>,
) -> PointerValue {
    let mut hir_types = hir_types.peekable();
    if hir_types.peek().is_none() {
        types
            .type_info_type
            .ptr_type(AddressSpace::Const)
            .ptr_type(AddressSpace::Const)
            .const_null()
    } else {
        let type_infos = hir_types
            .map(|ty| *global_type_info_lookup_table.get(&ty).unwrap())
            .collect::<Vec<PointerValue>>();

        let type_array_ir = types
            .type_info_type
            .ptr_type(AddressSpace::Const)
            .const_array(&type_infos);

        gen_global(module, &type_array_ir, "").as_pointer_value()
    }
}

/// Given a function, construct a pointer to a `MunTypeInfo` global that represents the return type
/// of the function; or `null` if the return type is empty.
fn gen_signature_return_type<D: IrDatabase>(
    db: &D,
    types: &AbiTypes,
    global_type_info_lookup_table: &HashMap<TypeInfo, PointerValue>,
    ret_type: Ty,
) -> PointerValue {
    gen_signature_return_type_from_type_info(
        db,
        types,
        global_type_info_lookup_table,
        if ret_type.is_empty() {
            None
        } else {
            Some(db.type_info(ret_type))
        },
    )
}

/// Given a function, construct a pointer to a `MunTypeInfo` global that represents the return type
/// of the function; or `null` if the return type is empty.
fn gen_signature_return_type_from_type_info<D: IrDatabase>(
    _db: &D,
    types: &AbiTypes,
    global_type_info_lookup_table: &HashMap<TypeInfo, PointerValue>,
    ret_type: Option<TypeInfo>,
) -> PointerValue {
    if let Some(ret_type) = ret_type {
        *global_type_info_lookup_table.get(&ret_type).unwrap()
    } else {
        types
            .type_info_type
            .ptr_type(AddressSpace::Const)
            .const_null()
    }
}

/// Construct a global that holds a reference to all functions. e.g.:
/// MunFunctionInfo[] info = { ... }
fn gen_function_info_array<'a, D: IrDatabase>(
    db: &D,
    module: &Module,
    types: &AbiTypes,
    global_type_info_lookup_table: &HashMap<TypeInfo, PointerValue>,
    functions: impl Iterator<Item = (&'a hir::Function, &'a FunctionValue)>,
) -> GlobalArrayValue {
    let function_infos: Vec<StructValue> = functions
        .filter(|(f, _)| f.visibility(db) == hir::Visibility::Public)
        .map(|(f, value)| {
            // Get the function from the cloned module and modify the linkage of the function.
            let value = module
                .get_function(value.get_name().to_str().unwrap())
                .unwrap();
            value.set_linkage(Linkage::Private);

            // Generate the signature from the function
            let signature =
                gen_signature_from_function(db, module, types, global_type_info_lookup_table, *f);

            // Generate the function info value
            types.function_info_type.const_named_struct(&[
                signature.into(),
                value.as_global_value().as_pointer_value().into(),
            ])
        })
        .collect();
    let num_functions = function_infos.len();
    let function_infos = types.function_info_type.const_array(&function_infos);

    GlobalArrayValue(
        gen_global(module, &function_infos, "fn.get_info.functions"),
        num_functions,
    )
}

/// Construct a global that holds a reference to all structs. e.g.:
/// MunStructInfo[] info = { ... }
fn gen_struct_info<D: IrDatabase>(
    db: &D,
    target: &TargetMachine,
    module: &Module,
    types: &AbiTypes,
    global_type_info_lookup_table: &mut HashMap<TypeInfo, PointerValue>,
    s: hir::Struct,
) -> StructValue {
    let name_str = intern_string(&module, &s.name(db).to_string());

    let fields = s.fields(db);
    let field_names = fields.iter().map(|field| field.name(db).to_string());
    let (field_names, num_fields) = gen_string_array(module, field_names);

    let field_types = expand_type_info(
        db,
        target,
        module,
        types,
        global_type_info_lookup_table,
        fields.iter().map(|field| db.type_info(field.ty(db))),
    );

    let target_data = target.get_target_data();
    let t = db.struct_ty(s);
    let field_offsets =
        (0..fields.len()).map(|idx| target_data.offset_of_element(&t, idx as u32).unwrap());
    let (field_offsets, _) = gen_u16_array(module, field_offsets);

    let field_sizes = fields
        .iter()
        .map(|field| target_data.get_store_size(&db.type_ir(field.ty(db))));
    let (field_sizes, _) = gen_u16_array(module, field_sizes);

    types.struct_info_type.const_named_struct(&[
        name_str.into(),
        field_names.into(),
        field_types.into(),
        field_offsets.into(),
        field_sizes.into(),
        module
            .get_context()
            .i16_type()
            .const_int(num_fields as u64, false)
            .into(),
    ])
}

/// Constructs a global from the specified list of strings
fn gen_string_array(
    module: &Module,
    strings: impl Iterator<Item = String>,
) -> (PointerValue, usize) {
    let str_type = module.get_context().i8_type().ptr_type(AddressSpace::Const);

    let mut strings = strings.peekable();
    if strings.peek().is_none() {
        (str_type.ptr_type(AddressSpace::Const).const_null(), 0)
    } else {
        let strings = strings
            .map(|s| intern_string(module, &s))
            .collect::<Vec<PointerValue>>();

        let strings_ir = str_type.const_array(&strings);
        (
            gen_global(module, &strings_ir, "").as_pointer_value(),
            strings.len(),
        )
    }
}

/// Constructs a global from the specified list of strings
fn gen_u16_array(module: &Module, integers: impl Iterator<Item = u64>) -> (PointerValue, usize) {
    let u16_type = module.get_context().i16_type();

    let mut integers = integers.peekable();
    if integers.peek().is_none() {
        (u16_type.ptr_type(AddressSpace::Const).const_null(), 0)
    } else {
        let integers = integers
            .map(|i| u16_type.const_int(i, false))
            .collect::<Vec<IntValue>>();

        let array_ir = u16_type.const_array(&integers);
        (
            gen_global(module, &array_ir, "").as_pointer_value(),
            integers.len(),
        )
    }
}

/// Construct a global from the specified value
fn gen_global(module: &Module, value: &dyn BasicValue, name: &str) -> GlobalValue {
    let global = module.add_global(value.as_basic_value_enum().get_type(), None, name);
    global.set_linkage(Linkage::Private);
    global.set_constant(true);
    global.set_unnamed_address(UnnamedAddress::Global);
    global.set_initializer(value);
    global
}

/// Generate the dispatch table information. e.g.:
/// ```c
/// MunDispatchTable dispatchTable = { ... }
/// ```
fn gen_dispatch_table<D: IrDatabase>(
    db: &D,
    module: &Module,
    types: &AbiTypes,
    global_type_info_lookup_table: &HashMap<TypeInfo, PointerValue>,
    dispatch_table: &DispatchTable,
) -> StructValue {
    // Generate a vector with all the function signatures
    let signatures: Vec<StructValue> = dispatch_table
        .entries()
        .iter()
        .map(|entry| {
            gen_signature_from_dispatch_entry(
                db,
                module,
                types,
                global_type_info_lookup_table,
                entry,
            )
        })
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
                .ptr_type(AddressSpace::Const)
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
    target: &TargetMachine,
    module: &Module,
    types: &HashSet<TypeInfo>,
    function_map: &HashMap<hir::Function, FunctionValue>,
    dispatch_table: &DispatchTable,
) {
    // Get all the types
    let abi_types = gen_abi_types(module.get_context());

    let mut global_type_info_lookup_table = HashMap::new();
    let num_types = types.len();
    let types = expand_type_info(
        db,
        target,
        module,
        &abi_types,
        &mut global_type_info_lookup_table,
        types.iter().cloned(),
    );

    let GlobalArrayValue(function_info, num_functions) = gen_function_info_array(
        db,
        module,
        &abi_types,
        &global_type_info_lookup_table,
        function_map.iter(),
    );

    // Construct the module info struct
    let module_info = abi_types.module_info_type.const_named_struct(&[
        intern_string(module, "").into(),
        function_info.as_pointer_value().into(),
        module
            .get_context()
            .i32_type()
            .const_int(num_functions as u64, false)
            .into(),
        types.into(),
        module
            .get_context()
            .i32_type()
            .const_int(num_types as u64, false)
            .into(),
    ]);

    // Construct the dispatch table struct
    let dispatch_table = gen_dispatch_table(
        db,
        module,
        &abi_types,
        &global_type_info_lookup_table,
        dispatch_table,
    );

    // Construct the actual `get_info` function
    gen_get_info_fn(db, module, &abi_types, module_info, dispatch_table);
}

/// Construct the actual `get_info` function.
fn gen_get_info_fn(
    db: &impl IrDatabase,
    module: &Module,
    abi_types: &AbiTypes,
    module_info: StructValue,
    dispatch_table: StructValue,
) {
    let context = module.get_context();
    let target = db.target();
    let str_type = context.i8_type().ptr_type(AddressSpace::Const);

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
        context.void_type().fn_type(
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
        module.add_function("get_info", get_symbols_type, Some(Linkage::DLLExport));

    if target.options.is_like_windows {
        get_symbols_fn.add_attribute(
            inkwell::attributes::AttributeLoc::Param(0),
            context.create_enum_attribute(Attribute::get_named_enum_kind_id("sret"), 1),
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
        str_type.ptr_type(AddressSpace::Const).const_null(),
    );
    builder.build_store(
        num_dependencies_addr,
        context.i32_type().const_int(0 as u64, false),
    );

    // Construct the return statement of the function.
    if target.options.is_like_windows {
        builder.build_return(None);
    } else {
        builder.build_return(Some(&builder.build_load(result_ptr, "")));
    }

    // Run the function optimizer on the generate function
    function::create_pass_manager(&module, db.optimization_lvl()).run_on(&get_symbols_fn);
}

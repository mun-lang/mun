use super::abi_types::{gen_abi_types, AbiTypes};
use crate::ir::dispatch_table::DispatchTable;
use crate::ir::function;
use crate::values::{BasicValue, GlobalValue};
use crate::IrDatabase;
use hir::{CallableDef, Ty, TypeCtor};
use inkwell::{
    attributes::Attribute,
    module::{Linkage, Module},
    types::StructType,
    values::{FunctionValue, IntValue, PointerValue, StructValue, UnnamedAddress},
    AddressSpace,
};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

pub type Guid = [u8; 16];

#[derive(Clone, Eq, Ord, PartialOrd, Debug)]
pub struct TypeInfo {
    pub guid: Guid,
    pub name: String,
}

impl Hash for TypeInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(&self.guid)
    }
}

impl PartialEq for TypeInfo {
    fn eq(&self, other: &Self) -> bool {
        self.guid == other.guid
    }
}

impl TypeInfo {
    fn from_name<S: AsRef<str>>(name: S) -> TypeInfo {
        TypeInfo {
            name: name.as_ref().to_string(),
            guid: md5::compute(name.as_ref()).0,
        }
    }
}

pub fn type_info_query(db: &impl IrDatabase, ty: Ty) -> TypeInfo {
    match ty {
        Ty::Apply(ctor) => match ctor.ctor {
            TypeCtor::Float => TypeInfo::from_name("@core::float"),
            TypeCtor::Int => TypeInfo::from_name("@core::int"),
            TypeCtor::Bool => TypeInfo::from_name("@core::bool"),
            TypeCtor::Struct(s) | TypeCtor::FnDef(CallableDef::Struct(s)) => {
                TypeInfo::from_name(s.name(db).to_string())
            }
            _ => unreachable!("{:?} unhandled", ctor),
        },
        _ => unreachable!(),
    }
}

/// Construct an IR `MunTypeInfo` struct value for the specified `TypeInfo`
fn type_info_ir(ty: &TypeInfo, module: &Module) -> StructValue {
    let context = module.get_context();
    let guid_values: [IntValue; 16] =
        array_init::array_init(|i| context.i8_type().const_int(u64::from(ty.guid[i]), false));
    context.const_struct(
        &[
            context.i8_type().const_array(&guid_values).into(),
            intern_string(module, &ty.name).into(),
        ],
        false,
    )
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
    function: hir::Function,
) -> StructValue {
    let name_str = intern_string(&module, &function.name(db).to_string());
    let _visibility = match function.visibility(db) {
        hir::Visibility::Public => 0,
        _ => 1,
    };
    let ret_type_ir = gen_signature_return_type(db, module, types, function);
    let (params_type_ir, num_params) = gen_signature_argument_types(db, module, types, function);

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

/// Given a function, construct a pointer to a `MunTypeInfo[]` global that represents the argument
/// types of the function; or `null` if the function has no arguments.
fn gen_signature_argument_types<D: IrDatabase>(
    db: &D,
    module: &Module,
    types: &AbiTypes,
    function: hir::Function,
) -> (PointerValue, usize) {
    let body = function.body(db);
    let infer = function.infer(db);

    let hir_types = body.params().iter().map(|(p, _)| infer[*p].clone());

    gen_type_info_array(db, module, types, hir_types)
}

/// Generates
fn gen_type_info_array<D: IrDatabase>(
    db: &D,
    module: &Module,
    types: &AbiTypes,
    hir_types: impl Iterator<Item = Ty>,
) -> (PointerValue, usize) {
    let mut hir_types = hir_types.peekable();
    if hir_types.peek().is_none() {
        (
            types
                .type_info_type
                .ptr_type(AddressSpace::Const)
                .const_null(),
            0,
        )
    } else {
        let type_infos = hir_types
            .map(|ty| type_info_ir(&db.type_info(ty), &module))
            .collect::<Vec<StructValue>>();

        let type_array_ir = types.type_info_type.const_array(&type_infos);
        (
            gen_global(module, &type_array_ir, "").as_pointer_value(),
            type_infos.len(),
        )
    }
}

/// Given a function, construct a pointer to a `MunTypeInfo` global that represents the return type
/// of the function; or `null` if the return type is empty.
fn gen_signature_return_type<D: IrDatabase>(
    db: &D,
    module: &Module,
    types: &AbiTypes,
    function: hir::Function,
) -> PointerValue {
    let sig = function.ty(db).callable_sig(db).unwrap();
    let ret_type = sig.ret().clone();
    if ret_type.is_empty() {
        types
            .type_info_type
            .ptr_type(AddressSpace::Const)
            .const_null()
    } else {
        let ret_type_const = type_info_ir(&db.type_info(ret_type), &module);
        gen_global(module, &ret_type_const, "").as_pointer_value()
    }
}

/// Construct a global that holds a reference to all functions. e.g.:
/// MunFunctionInfo[] info = { ... }
fn gen_function_info_array<'a, D: IrDatabase>(
    db: &D,
    types: &AbiTypes,
    module: &Module,
    functions: impl Iterator<Item = (&'a hir::Function, &'a FunctionValue)>,
) -> GlobalValue {
    let function_infos: Vec<StructValue> = functions
        .map(|(f, value)| {
            // Get the function from the cloned module and modify the linkage of the function.
            let value = module
                .get_function(value.get_name().to_str().unwrap())
                .unwrap();
            value.set_linkage(Linkage::Private);

            // Generate the signature from the function
            let signature = gen_signature_from_function(db, module, types, *f);

            // Generate the function info value
            types.function_info_type.const_named_struct(&[
                signature.into(),
                value.as_global_value().as_pointer_value().into(),
            ])
        })
        .collect();
    let function_infos = types.function_info_type.const_array(&function_infos);
    gen_global(module, &function_infos, "fn.get_info.functions")
}

/// Construct a global that holds a reference to all structs. e.g.:
/// MunStructInfo[] info = { ... }
fn gen_struct_info_array<'a, D: IrDatabase>(
    db: &D,
    types: &AbiTypes,
    module: &Module,
    structs: impl Iterator<Item = (&'a hir::Struct, &'a StructType)>,
) -> GlobalValue {
    let struct_infos: Vec<StructValue> = structs
        .map(|(s, _)| {
            let name_str = intern_string(&module, &s.name(db).to_string());

            let fields = s.fields(db);
            let field_types = fields.iter().map(|field| field.ty(db));
            let (fields, num_fields) = gen_type_info_array(db, module, types, field_types);

            types.struct_info_type.const_named_struct(&[
                name_str.into(),
                fields.into(),
                module
                    .get_context()
                    .i16_type()
                    .const_int(num_fields as u64, false)
                    .into(),
            ])
        })
        .collect();

    let struct_infos = types.struct_info_type.const_array(&struct_infos);
    gen_global(module, &struct_infos, "fn.get_info.structs")
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
    types: &AbiTypes,
    module: &Module,
    dispatch_table: &DispatchTable,
) -> StructValue {
    // Generate a vector with all the function signatures
    let signatures: Vec<StructValue> = dispatch_table
        .entries()
        .iter()
        .map(|f| gen_signature_from_function(db, module, types, *f))
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
    function_map: &HashMap<hir::Function, FunctionValue>,
    struct_map: &HashMap<hir::Struct, StructType>,
    dispatch_table: &DispatchTable,
    module: &Module,
) {
    // Get all the types
    let abi_types = gen_abi_types(module.get_context());

    // Construct the module info struct
    let module_info = abi_types.module_info_type.const_named_struct(&[
        intern_string(module, "").into(),
        gen_function_info_array(db, &abi_types, module, function_map.iter())
            .as_pointer_value()
            .into(),
        module
            .get_context()
            .i32_type()
            .const_int(function_map.len() as u64, false)
            .into(),
        gen_struct_info_array(db, &abi_types, module, struct_map.iter())
            .as_pointer_value()
            .into(),
        module
            .get_context()
            .i32_type()
            .const_int(struct_map.len() as u64, false)
            .into(),
    ]);

    // Construct the dispatch table struct
    let dispatch_table = gen_dispatch_table(db, &abi_types, module, dispatch_table);

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

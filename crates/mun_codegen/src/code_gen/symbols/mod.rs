use std::{collections::HashSet, ffi::CString};

use inkwell::{
    attributes::Attribute,
    module::Linkage,
    types::AnyType,
    values::{GlobalValue, PointerValue},
    AddressSpace,
};
use ir_type_builder::TypeIdBuilder;
use itertools::Itertools;
use mun_abi as abi;
use mun_hir::{HirDatabase, TyKind};

use crate::{
    ir::{
        dispatch_table::{DispatchTable, DispatchableFunction},
        function,
        ty::{guid_from_struct, HirTypeCache},
        type_table::TypeTable,
        types::{self as ir, AbiBuilder},
    },
    type_info::HasStaticTypeId,
};

mod ir_type_builder;

fn gen_prototype_from_function<'ink>(
    db: &dyn HirDatabase,
    abi: &AbiBuilder<'_, 'ink>,
    function: mun_hir::Function,
    hir_types: &HirTypeCache<'_, 'ink>,
    type_ids: &TypeIdBuilder<'_, '_, 'ink>,
) -> ir::FunctionPrototype<'ink> {
    let name = function.full_name(db);
    let name = abi.types.intern_c_str(
        abi.module,
        &format!("fn_sig::<{name}>::name"),
        &CString::new(name.clone()).expect("function prototype name is not a valid CString"),
    );

    let signature = function.ty(db).callable_sig(db).unwrap();
    let return_type = if signature.ret().is_empty() {
        type_ids.construct_from_type_id(<() as HasStaticTypeId>::type_id())
    } else {
        type_ids.construct_from_type_id(&hir_types.type_id(signature.ret()))
    };
    let arg_types: Vec<_> = signature
        .params()
        .iter()
        .map(|ty| type_ids.construct_from_type_id(&hir_types.type_id(ty)))
        .collect();
    let arg_types = abi.types.private_type_id_array(
        abi.module,
        &format!("fn_sig::<{name}>::arg_types"),
        &arg_types,
        true,
    );

    ir::FunctionPrototype {
        name,
        signature: ir::FunctionSignature {
            arg_types,
            return_type,
            num_arg_types: signature.params().len() as u16,
        },
    }
}

fn gen_prototype_from_dispatch_entry<'ink>(
    abi: &AbiBuilder<'_, 'ink>,
    function: &DispatchableFunction,
    type_ids: &TypeIdBuilder<'_, '_, 'ink>,
) -> ir::FunctionPrototype<'ink> {
    let name = abi.types.intern_c_str(
        abi.module,
        &format!("fn_sig::<{}>::name", function.prototype.name),
        &CString::new(function.prototype.name.clone())
            .expect("function prototype name is not a valid CString"),
    );
    let return_type = type_ids.construct_from_type_id(&function.prototype.ret_type);
    let arg_types: Vec<_> = function
        .prototype
        .arg_types
        .iter()
        .map(|type_info| type_ids.construct_from_type_id(type_info))
        .collect();
    let arg_types = abi.types.private_type_id_array(
        abi.module,
        &format!("{}_param_types", function.prototype.name),
        &arg_types,
        true,
    );

    ir::FunctionPrototype {
        name,
        signature: ir::FunctionSignature {
            arg_types,
            return_type,
            num_arg_types: function.prototype.arg_types.len() as u16,
        },
    }
}

fn get_type_definition_array<'ink>(
    db: &dyn HirDatabase,
    abi: &AbiBuilder<'_, 'ink>,
    types: impl Iterator<Item = mun_hir::Ty>,
    hir_types: &HirTypeCache<'_, 'ink>,
    type_ids: &TypeIdBuilder<'_, '_, 'ink>,
) -> PointerValue<'ink> {
    let values: Vec<_> = types
        .sorted_by_cached_key(|type_info| match type_info.interned() {
            TyKind::Struct(value) => value.full_name(db),
            _ => unreachable!("unsupported export type"),
        })
        .map(|type_info| match type_info.interned() {
            TyKind::Struct(value) => {
                let llvm_type = hir_types.get_struct_type(*value);
                let name = value.full_name(db);
                ir::TypeDefinition {
                    name: abi.types.intern_c_str(
                        abi.module,
                        &format!("type_info::<{name}>::name"),
                        &CString::new(name).expect("typename is not a valid CString"),
                    ),
                    size_in_bits: abi
                        .target_data
                        .get_bit_size(&llvm_type)
                        .try_into()
                        .expect("could not convert size in bits to smaller size"),
                    alignment: abi
                        .target_data
                        .get_abi_alignment(&llvm_type)
                        .try_into()
                        .expect("could not convert alignment to smaller size"),
                    data: gen_struct_info(db, *value, abi, hir_types, type_ids),
                }
            }
            _ => unreachable!("unsupported export type"),
        })
        .collect();

    abi.types
        .private_type_definition_array(abi.module, "fn.get_info.types", &values, true)
}

fn gen_struct_info<'ink>(
    db: &dyn HirDatabase,
    hir_struct: mun_hir::Struct,
    abi: &AbiBuilder<'_, 'ink>,
    hir_types: &HirTypeCache<'_, 'ink>,
    type_ids: &TypeIdBuilder<'_, '_, 'ink>,
) -> ir::StructDefinition<'ink> {
    let struct_ir = hir_types.get_struct_type(hir_struct);
    let name = hir_struct.full_name(db);
    let fields = hir_struct.fields(db);

    let field_names: Vec<_> = fields
        .iter()
        .enumerate()
        .map(|(index, field)| {
            abi.types.intern_c_str(
                abi.module,
                &format!("struct_info::<{name}>::field_names.{index}"),
                &CString::new(field.name(db).to_string())
                    .expect("field name is not a valid CString"),
            )
        })
        .collect();
    let field_names = abi.types.private_pointer_array(
        abi.module,
        &format!("struct_info::<{name}>::field_names"),
        abi.context.i8_type().ptr_type(AddressSpace::default()),
        &field_names,
        true,
    );

    let field_types: Vec<_> = fields
        .iter()
        .map(|field| type_ids.construct_from_type_id(&hir_types.type_id(&field.ty(db))))
        .collect();
    let field_types = abi.types.private_type_id_array(
        abi.module,
        &format!("struct_info::<{name}>::field_types"),
        &field_types,
        true,
    );

    let field_offsets: Vec<_> = fields
        .iter()
        .enumerate()
        .map(|(index, _)| {
            abi.target_data
                .offset_of_element(&struct_ir, index as u32)
                .expect("struct field must have an offset") as u16
        })
        .collect();
    let field_offsets = abi.types.private_u16_array(
        abi.module,
        &format!("struct_info::<{name}>::field_offsets"),
        &field_offsets,
        true,
    );

    ir::StructDefinition {
        guid: guid_from_struct(db, hir_struct),
        field_names,
        field_types,
        field_offsets,
        num_fields: fields
            .len()
            .try_into()
            .expect("could not convert num_fields to smaller bit size"),
        memory_kind: hir_struct.data(db).memory_kind,
    }
}

fn get_function_definition_array<'ink, 'a>(
    db: &dyn HirDatabase,
    abi: &AbiBuilder<'_, 'ink>,
    functions: impl Iterator<Item = &'a mun_hir::Function>,
    hir_types: &HirTypeCache<'_, 'ink>,
    type_ids: &TypeIdBuilder<'_, '_, 'ink>,
) -> GlobalValue<'ink> {
    let values: Vec<_> = functions
        .sorted_by_cached_key(|function| function.full_name(db))
        .map(|function| {
            let name = function.name(db).to_string();
            let value = abi
                .module
                .get_function(&format!("{name}_wrapper"))
                .or_else(|| abi.module.get_function(&name))
                .expect("generated function must exist in the assembly module");
            value.set_linkage(Linkage::Private);

            ir::FunctionDefinition {
                prototype: gen_prototype_from_function(db, abi, *function, hir_types, type_ids),
                fn_ptr: value
                    .as_global_value()
                    .as_pointer_value()
                    .const_cast(abi.context.i8_type().ptr_type(AddressSpace::default())),
            }
        })
        .collect();

    abi.types
        .private_function_definition_array(abi.module, "fn.get_info.functions", &values)
}

fn gen_type_lut<'ink>(
    abi: &AbiBuilder<'_, 'ink>,
    type_table: &TypeTable<'ink>,
    type_ids: &TypeIdBuilder<'_, '_, 'ink>,
) -> ir::TypeLut<'ink> {
    let values: Vec<_> = type_table
        .entries()
        .iter()
        .map(|ty| type_ids.construct_from_type_id(ty))
        .collect();
    let type_ids =
        abi.types
            .private_type_id_array(abi.module, "fn.get_info.typeLut.typeIds", &values, false);

    let type_names: Vec<_> = type_table
        .entries()
        .iter()
        .map(|ty| {
            abi.types.intern_c_str(
                abi.module,
                &ty.name,
                &CString::new(ty.name.as_str())
                    .expect("unable to create CString from typeinfo name"),
            )
        })
        .collect();
    let byte_ptr = abi.context.i8_type().ptr_type(AddressSpace::default());
    let type_names = abi.types.private_pointer_array(
        abi.module,
        "fn.get_info.typeLut.typeNames",
        byte_ptr,
        &type_names,
        false,
    );

    let pointer_table_type = byte_ptr.ptr_type(AddressSpace::default());
    let type_ptrs = TypeTable::find_global(abi.module).map_or_else(
        || pointer_table_type.const_null(),
        |global| global.as_pointer_value().const_cast(pointer_table_type),
    );

    ir::TypeLut {
        type_ids,
        type_ptrs,
        type_names,
        num_entries: type_table.num_types().try_into().expect("too many types"),
    }
}

fn gen_dispatch_table<'ink>(
    abi: &AbiBuilder<'_, 'ink>,
    dispatch_table: &DispatchTable<'ink>,
    type_ids: &TypeIdBuilder<'_, '_, 'ink>,
) -> ir::DispatchTable<'ink> {
    let prototypes: Vec<_> = dispatch_table
        .entries()
        .iter()
        .map(|entry| gen_prototype_from_dispatch_entry(abi, entry, type_ids))
        .collect();
    let prototypes = abi.types.private_function_prototype_array(
        abi.module,
        "fn.get_info.dispatchTable.signatures",
        &prototypes,
    );

    let byte_ptr = abi.context.i8_type().ptr_type(AddressSpace::default());
    let pointer_table_type = byte_ptr.ptr_type(AddressSpace::default());
    let fn_ptrs = dispatch_table.global_value().map_or_else(
        || pointer_table_type.const_null(),
        |_| {
            abi.module
                .get_global("dispatchTable")
                .expect("dispatch table global must exist in the assembly module")
                .as_pointer_value()
                .const_cast(pointer_table_type)
        },
    );

    ir::DispatchTable {
        prototypes,
        fn_ptrs,
        num_entries: dispatch_table.entries().len() as u32,
    }
}

/// Generates the runtime reflection entry points for one assembly module.
#[expect(
    clippy::too_many_arguments,
    reason = "the inputs are distinct assembly products"
)]
pub(super) fn gen_reflection_ir<'db, 'ink>(
    db: &'db dyn HirDatabase,
    abi: &AbiBuilder<'_, 'ink>,
    module_name: &str,
    function_definitions: &HashSet<mun_hir::Function>,
    type_definitions: &HashSet<mun_hir::Ty>,
    dispatch_table: &DispatchTable<'ink>,
    type_table: &TypeTable<'ink>,
    hir_types: &HirTypeCache<'db, 'ink>,
    optimization_level: inkwell::OptimizationLevel,
    dependencies: Vec<String>,
) {
    let type_ids = TypeIdBuilder::new(abi);
    let functions =
        get_function_definition_array(db, abi, function_definitions.iter(), hir_types, &type_ids);
    let types = get_type_definition_array(
        db,
        abi,
        type_definitions.iter().cloned(),
        hir_types,
        &type_ids,
    );
    let functions = functions.as_pointer_value().const_cast(
        abi.types
            .function_definition_type()
            .ptr_type(AddressSpace::default()),
    );

    let module_info = ir::ModuleInfo {
        path: abi.types.intern_c_str(
            abi.module,
            "module_info::path",
            &CString::new(module_name).expect("module path is not a valid CString"),
        ),
        functions,
        num_functions: function_definitions.len() as u32,
        types,
        num_types: type_definitions.len() as u32,
    };
    let dispatch_table = gen_dispatch_table(abi, dispatch_table, &type_ids);
    let type_lut = gen_type_lut(abi, type_table, &type_ids);

    gen_get_info_fn(
        db,
        abi,
        &module_info,
        &dispatch_table,
        &type_lut,
        optimization_level,
        dependencies,
    );
    gen_set_allocator_handle_fn(abi);
    gen_get_version_fn(abi);
}

fn gen_get_info_fn<'ink>(
    db: &dyn HirDatabase,
    abi: &AbiBuilder<'_, 'ink>,
    module_info: &ir::ModuleInfo<'ink>,
    dispatch_table: &ir::DispatchTable<'ink>,
    type_lut: &ir::TypeLut<'ink>,
    optimization_level: inkwell::OptimizationLevel,
    dependencies: Vec<String>,
) {
    let is_windows = db.target().options.is_like_windows;
    let function_type = abi.types.get_info_function_type(is_windows);
    let function = abi.module.add_function(
        abi::GET_INFO_FN_NAME,
        function_type,
        Some(Linkage::DLLExport),
    );

    if is_windows {
        let attribute = abi.context.create_type_attribute(
            Attribute::get_named_enum_kind_id("sret"),
            abi.types.assembly_info_type().as_any_type_enum(),
        );
        function.add_attribute(inkwell::attributes::AttributeLoc::Param(0), attribute);
    }

    let dependency_values: Vec<_> = dependencies
        .iter()
        .enumerate()
        .map(|(index, name)| {
            abi.types.intern_c_str(
                abi.module,
                &format!("dependency{index}"),
                &CString::new(name.as_str()).expect("dependency name is not a valid CString"),
            )
        })
        .collect();
    let byte_ptr = abi.context.i8_type().ptr_type(AddressSpace::default());
    let dependencies_ptr = abi.types.private_pointer_array(
        abi.module,
        "dependencies",
        byte_ptr,
        &dependency_values,
        true,
    );
    let assembly_info = abi.types.assembly_info_value(
        module_info,
        dispatch_table,
        type_lut,
        dependencies_ptr,
        dependencies
            .len()
            .try_into()
            .expect("too many dependencies"),
    );

    let builder = abi.context.create_builder();
    let body = abi.context.append_basic_block(function, "body");
    builder.position_at_end(body);
    if is_windows {
        builder.build_store(
            function
                .get_nth_param(0)
                .expect("sret function must receive a result pointer")
                .into_pointer_value(),
            assembly_info,
        );
        builder.build_return(None);
    } else {
        builder.build_return(Some(&assembly_info));
    }

    function::create_pass_manager(abi.module, optimization_level).run_on(&function);
}

fn gen_set_allocator_handle_fn(abi: &AbiBuilder<'_, '_>) {
    let pointer_type = abi.context.i8_type().ptr_type(AddressSpace::default());
    let function_type = abi
        .context
        .void_type()
        .fn_type(&[pointer_type.into()], false);
    let function = abi.module.add_function(
        abi::SET_ALLOCATOR_HANDLE_FN_NAME,
        function_type,
        Some(Linkage::DLLExport),
    );
    let builder = abi.context.create_builder();
    let body = abi.context.append_basic_block(function, "body");
    builder.position_at_end(body);

    if let Some(global) = abi.module.get_global("allocatorHandle") {
        builder.build_store(
            global.as_pointer_value(),
            function
                .get_nth_param(0)
                .expect("allocator setter must receive the allocator handle"),
        );
    }
    builder.build_return(None);
}

fn gen_get_version_fn(abi: &AbiBuilder<'_, '_>) {
    let function_type = abi.context.i32_type().fn_type(&[], false);
    let function = abi.module.add_function(
        abi::GET_VERSION_FN_NAME,
        function_type,
        Some(Linkage::DLLExport),
    );
    let builder = abi.context.create_builder();
    let body = abi.context.append_basic_block(function, "body");
    builder.position_at_end(body);
    builder.build_return(Some(
        &abi.context
            .i32_type()
            .const_int(u64::from(abi::ABI_VERSION), false),
    ));
}

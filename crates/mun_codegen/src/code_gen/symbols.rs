use crate::ir::function;
use crate::IrDatabase;
use inkwell::attributes::Attribute;
use inkwell::values::{IntValue, PointerValue};
use inkwell::{
    module::{Linkage, Module},
    values::{FunctionValue, StructValue},
    AddressSpace,
};
use mun_hir::{Ty, TypeCtor};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

pub type Guid = [u8; 16];

#[derive(Clone, Eq, Ord, PartialOrd, PartialEq, Debug)]
pub struct TypeInfo {
    pub guid: Guid,
    pub name: String,
}

impl Hash for TypeInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(&self.guid)
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

pub fn type_info_query(_db: &impl IrDatabase, ty: Ty) -> TypeInfo {
    match ty {
        Ty::Apply(ctor) => match ctor.ctor {
            TypeCtor::Float => TypeInfo::from_name("@core::float"),
            TypeCtor::Int => TypeInfo::from_name("@core::int"),
            TypeCtor::Bool => TypeInfo::from_name("@core::bool"),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    }
}

fn type_info_ir(ty: &TypeInfo, module: &Module) -> StructValue {
    let context = module.get_context();
    let guid_values: [IntValue; 16] =
        array_init::array_init(|i| context.i8_type().const_int(ty.guid[i] as u64, false));
    context.const_struct(
        &[
            context.i8_type().const_array(&guid_values).into(),
            intern_string(module, &ty.name).into(),
        ],
        false,
    )
}

fn intern_string(module: &Module, str: &str) -> PointerValue {
    let value = module.get_context().const_string(str, true);
    let global_value = module.add_global(value.get_type(), None, "");
    global_value.set_linkage(Linkage::Internal);
    global_value.set_initializer(&value);
    global_value.as_pointer_value()
}

pub(super) fn gen_symbols(
    db: &impl IrDatabase,
    function_map: &HashMap<mun_hir::Function, FunctionValue>,
    module: &Module,
) {
    let context = module.get_context();
    let str_type = context.i8_type().ptr_type(AddressSpace::Const);
    let target = db.target();

    let guid_type = context.i8_type().array_type(16);
    let privacy_type = context.i8_type();

    let type_info_type = context.opaque_struct_type("struct.MunTypeInfo");
    type_info_type.set_body(
        &[
            guid_type.into(), // guid
            str_type.into(),  // name
        ],
        false,
    );

    let method_info_type = context.opaque_struct_type("struct.MunFunctionInfo");
    method_info_type.set_body(
        &[
            str_type.into(),                                     // name
            type_info_type.ptr_type(AddressSpace::Const).into(), // arg_types
            type_info_type.ptr_type(AddressSpace::Const).into(), // return_type
            context
                .void_type()
                .fn_type(&[], false)
                .ptr_type(AddressSpace::Const)
                .into(), // fn_ptr
            context.i16_type().into(),                           // num_arg_types
            privacy_type.into(),                                 // privacy
        ],
        false,
    );

    let module_info_type = context.opaque_struct_type("struct.MunModuleInfo");
    module_info_type.set_body(
        &[
            method_info_type.ptr_type(AddressSpace::Const).into(),
            context.i32_type().into(),
        ],
        false,
    );

    let method_infos: Vec<StructValue> = function_map
        .iter()
        .map(|(f, value)| {
            // Get the function from the clone module.
            let value = module
                .get_function(value.get_name().to_str().unwrap())
                .unwrap();
            value.set_linkage(Linkage::Private);

            // Intern the name of the function
            let name_str = intern_string(&module, &f.name(db).to_string());

            // Get the return value type
            let body = f.body(db);
            let infer = f.infer(db);
            let ret_type = infer[body.body_expr()].clone();
            let ret_type_ir: PointerValue = if ret_type.is_empty() {
                type_info_type.ptr_type(AddressSpace::Const).const_null()
            } else {
                let ret_type_const = type_info_ir(&db.type_info(ret_type), &module);
                let ret_type_ir = module.add_global(ret_type_const.get_type(), None, "");
                ret_type_ir.set_linkage(Linkage::Internal);
                ret_type_ir.set_initializer(&ret_type_const);
                ret_type_ir.as_pointer_value()
            };

            // Get the argument types
            let params_type_ir: PointerValue = if body.params().is_empty() {
                type_info_type.ptr_type(AddressSpace::Const).const_null()
            } else {
                let params_type_array_ir = type_info_type.const_array(
                    &body
                        .params()
                        .iter()
                        .map(|(p, _)| type_info_ir(&db.type_info(infer[*p].clone()), &module))
                        .collect::<Vec<StructValue>>(),
                );
                let params_type_ir = module.add_global(params_type_array_ir.get_type(), None, "");
                params_type_ir.set_linkage(Linkage::Internal);
                params_type_ir.set_initializer(&params_type_array_ir);
                params_type_ir.as_pointer_value()
            };

            context.const_struct(
                &[
                    name_str.into(),
                    params_type_ir.into(),
                    ret_type_ir.into(),
                    value.as_global_value().as_pointer_value().into(),
                    context
                        .i16_type()
                        .const_int(body.params().len() as u64, false)
                        .into(),
                    context.i8_type().const_int(0, false).into(),
                ],
                false,
            )
        })
        .collect();

    let method_info_array = method_info_type.const_array(&method_infos);
    let method_info = module.add_global(method_info_array.get_type(), None, "");
    method_info.set_linkage(Linkage::Internal);
    method_info.set_initializer(&method_info_array);

    let get_symbols_type = if target.options.is_like_windows {
        context.void_type().fn_type(
            &[module_info_type.ptr_type(AddressSpace::Generic).into()],
            false,
        )
    } else {
        module_info_type.fn_type(&[], false)
    };

    let get_symbols_fn =
        module.add_function("get_symbols", get_symbols_type, Some(Linkage::DLLExport));

    if target.options.is_like_windows {
        get_symbols_fn.add_attribute(
            inkwell::attributes::AttributeLoc::Param(0),
            context.create_enum_attribute(Attribute::get_named_enum_kind_id("sret"), 1),
        );
    }

    let builder = db.context().create_builder();
    let body_ir = get_symbols_fn.append_basic_block("body");
    builder.position_at_end(&body_ir);

    let result_ptr = if target.options.is_like_windows {
        get_symbols_fn
            .get_nth_param(0)
            .unwrap()
            .into_pointer_value()
    } else {
        builder.build_alloca(module_info_type, "")
    };

    // Get access to the structs internals
    let function_info_addr = unsafe { builder.build_struct_gep(result_ptr, 0, "") };
    let function_count_addr = unsafe { builder.build_struct_gep(result_ptr, 1, "") };

    // Fill them with values
    builder.build_store(function_info_addr, method_info.as_pointer_value());
    builder.build_store(
        function_count_addr,
        context
            .i32_type()
            .const_int(method_infos.len() as u64, false),
    );

    if target.options.is_like_windows {
        builder.build_return(None);
    } else {
        builder.build_return(Some(&builder.build_load(result_ptr, "")));
    }

    function::create_pass_manager(&module, db.optimization_lvl()).run_on(&get_symbols_fn);

    //println!("{}", module.print_to_string().to_string())
}

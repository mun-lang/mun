use crate::ir::{
    abi_types::gen_abi_types,
    adt,
    dispatch_table::{DispatchTable, DispatchTableBuilder},
    function, intrinsics,
    type_table::{TypeTable, TypeTableBuilder},
};
use crate::{CodeGenParams, IrDatabase};
use hir::{FileId, ModuleDef};
use inkwell::{
    values::{FunctionValue, UnnamedAddress},
    AddressSpace,
};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ModuleIR {
    /// The original source file
    pub file_id: FileId,

    /// A mapping from HIR functions to LLVM IR values
    pub functions: HashMap<hir::Function, FunctionValue>,

    /// The dispatch table
    pub dispatch_table: DispatchTable,

    /// The type table
    pub type_table: TypeTable,
}

/// Generates IR for the specified file
pub(crate) fn ir_query(db: &impl IrDatabase, file_id: FileId) -> Arc<ModuleIR> {
    let llvm_module = db.module();
    let abi_types = gen_abi_types(llvm_module.get_context());

    // Collect all intrinsic functions and wrapper function.
    // Use a `BTreeMap` to guarantee deterministically ordered output.
    let mut intrinsics_map = BTreeMap::new();
    let mut wrappers = BTreeMap::new();
    for def in db.module_data(file_id).definitions() {
        match def {
            ModuleDef::Function(f) if !f.is_extern(db) => {
                let fn_sig = f.ty(db).callable_sig(db).unwrap();
                let body = f.body(db);
                let infer = f.infer(db);

                intrinsics::collect_fn_body(db, &llvm_module, &mut intrinsics_map, &body, &infer);

                if !f.data(db).visibility().is_private() && !fn_sig.marshallable(db) {
                    intrinsics::collect_wrapper_body(&llvm_module, &mut intrinsics_map);

                    // Generate wrapper function
                    let wrapper_fun = function::gen_signature(
                        db,
                        *f,
                        &llvm_module,
                        CodeGenParams {
                            make_marshallable: true,
                        },
                    );
                    wrappers.insert(*f, wrapper_fun);
                }
            }
            ModuleDef::Function(_) => (), // TODO: Extern types?
            ModuleDef::Struct(_) => (),
            ModuleDef::BuiltinType(_) => (),
        }
    }

    // Collect all used types
    let mut type_table_builder =
        TypeTableBuilder::new(db, &llvm_module, &abi_types, intrinsics_map.keys());

    for def in db.module_data(file_id).definitions() {
        match def {
            ModuleDef::Struct(s) => {
                adt::gen_struct_decl(db, *s);
                type_table_builder.collect_struct(*s);
            }
            ModuleDef::Function(f) => {
                type_table_builder.collect_fn(*f);
            }
            ModuleDef::BuiltinType(_) => (),
        }
    }

    let type_table = type_table_builder.build();

    // Generate all the function signatures
    let mut functions = HashMap::new();
    let mut dispatch_table_builder = DispatchTableBuilder::new(db, &llvm_module, intrinsics_map);
    for def in db.module_data(file_id).definitions() {
        // TODO: Remove once we have more ModuleDef variants
        #[allow(clippy::single_match)]
        match def {
            ModuleDef::Function(f) if !f.is_extern(db) => {
                // Construct the function signature
                let fun = function::gen_signature(
                    db,
                    *f,
                    &llvm_module,
                    CodeGenParams {
                        make_marshallable: false,
                    },
                );
                functions.insert(*f, fun);

                // Add calls to the dispatch table
                let body = f.body(db);
                let infer = f.infer(db);
                dispatch_table_builder.collect_body(&body, &infer);
            }
            _ => {}
        }
    }

    // Construct requirements for generating the bodies
    let dispatch_table = dispatch_table_builder.finalize(&functions);
    let fn_pass_manager = function::create_pass_manager(&llvm_module, db.optimization_lvl());

    // Create the allocator handle global value
    let allocator_handle_global = if dispatch_table.has_intrinsic(&crate::intrinsics::new) {
        let allocator_handle_global_type = db.context().i8_type().ptr_type(AddressSpace::Generic);
        let allocator_handle_global =
            llvm_module.add_global(allocator_handle_global_type, None, "allocatorHandle");
        allocator_handle_global.set_initializer(&allocator_handle_global_type.const_null());
        allocator_handle_global.set_linkage(inkwell::module::Linkage::Private);
        allocator_handle_global.set_unnamed_address(UnnamedAddress::Global);
        Some(allocator_handle_global)
    } else {
        None
    };

    // Generate the function bodies
    for (hir_function, llvm_function) in functions.iter() {
        function::gen_body(
            db,
            *hir_function,
            *llvm_function,
            &llvm_module,
            &functions,
            &dispatch_table,
            &type_table,
            allocator_handle_global,
        );
        fn_pass_manager.run_on(llvm_function);
    }

    for (hir_function, llvm_function) in wrappers.iter() {
        function::gen_wrapper_body(
            db,
            *hir_function,
            *llvm_function,
            &llvm_module,
            &functions,
            &dispatch_table,
            &type_table,
            allocator_handle_global,
        );
        fn_pass_manager.run_on(llvm_function);
    }

    // Filter private methods
    let mut api: HashMap<hir::Function, FunctionValue> = functions
        .into_iter()
        .filter(|(f, _)| f.visibility(db) != hir::Visibility::Private)
        .collect();

    // Replace non-marshallable functions with their marshallable wrappers
    for (hir_function, llvm_function) in wrappers {
        api.insert(hir_function, llvm_function);
    }

    Arc::new(ModuleIR {
        file_id,
        functions: api,
        dispatch_table,
        type_table,
    })
}

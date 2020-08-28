use super::{
    dispatch_table::{DispatchTable, DispatchTableBuilder},
    intrinsics,
    type_table::{TypeTable, TypeTableBuilder},
};
use crate::code_gen::CodeGenContext;
use crate::value::{IrTypeContext, IrValueContext};
use hir::ModuleDef;
use inkwell::{module::Module, types::PointerType, values::UnnamedAddress, AddressSpace};
use std::collections::BTreeMap;

/// The IR generated for a group of files. It is used to generate IR for all of the group's files
/// and the resulting `Assembly`'s symbols.
#[derive(Debug, PartialEq, Eq)]
pub struct FileGroupIR<'ink> {
    /// The LLVM module that contains the IR
    pub(crate) llvm_module: Module<'ink>,
    /// The dispatch table
    pub(crate) dispatch_table: DispatchTable<'ink>,
    /// The type table
    pub(crate) type_table: TypeTable<'ink>,
    /// The allocator handle, if it exists
    pub(crate) allocator_handle_type: Option<PointerType<'ink>>,
}

/// Generates IR that is shared among the group's files.
/// TODO: Currently, a group always consists of a single file. Need to add support for multiple
///  files using something like `FileGroupId`.
pub(crate) fn gen_file_group_ir<'db, 'ink>(
    code_gen: &CodeGenContext<'db, 'ink>,
    file_id: hir::FileId,
) -> FileGroupIR<'ink> {
    let llvm_module = code_gen.context.create_module("group_name");

    // Use a `BTreeMap` to guarantee deterministically ordered output.
    let mut intrinsics_map = BTreeMap::new();
    let mut needs_alloc = false;

    // Collect all intrinsic functions, wrapper function, and generate struct declarations.
    for def in code_gen.db.module_data(file_id).definitions() {
        match def {
            ModuleDef::Function(f) if !f.is_extern(code_gen.db) => {
                intrinsics::collect_fn_body(
                    &code_gen.context,
                    code_gen.target_machine.get_target_data(),
                    code_gen.db,
                    &mut intrinsics_map,
                    &mut needs_alloc,
                    &f.body(code_gen.db),
                    &f.infer(code_gen.db),
                );

                let fn_sig = f.ty(code_gen.db).callable_sig(code_gen.db).unwrap();
                if !f.data(code_gen.db).visibility().is_private()
                    && !fn_sig.marshallable(code_gen.db)
                {
                    intrinsics::collect_wrapper_body(
                        &code_gen.context,
                        code_gen.target_machine.get_target_data(),
                        &mut intrinsics_map,
                        &mut needs_alloc,
                    );
                }
            }
            ModuleDef::Function(_) => (), // TODO: Extern types?
            ModuleDef::Struct(_) => (),
            ModuleDef::BuiltinType(_) => (),
            ModuleDef::TypeAlias(_) => (),
        }
    }

    // Collect all exposed functions' bodies.
    let mut dispatch_table_builder = DispatchTableBuilder::new(
        code_gen.context,
        code_gen.target_machine.get_target_data(),
        code_gen.db,
        &llvm_module,
        &intrinsics_map,
        &code_gen.hir_types,
    );
    for def in code_gen.db.module_data(file_id).definitions() {
        if let ModuleDef::Function(f) = def {
            if !f.data(code_gen.db).visibility().is_private() && !f.is_extern(code_gen.db) {
                let body = f.body(code_gen.db);
                let infer = f.infer(code_gen.db);
                dispatch_table_builder.collect_body(&body, &infer);
            }
        }
    }

    let dispatch_table = dispatch_table_builder.build();

    let target_data = code_gen.target_machine.get_target_data();
    let type_context = IrTypeContext {
        context: &code_gen.context,
        target_data: &target_data,
        struct_types: &code_gen.rust_types,
    };
    let value_context = IrValueContext {
        type_context: &type_context,
        context: &code_gen.context,
        module: &llvm_module,
    };
    let mut type_table_builder = TypeTableBuilder::new(
        code_gen.db,
        code_gen.target_machine.get_target_data(),
        &value_context,
        intrinsics_map.keys(),
        &dispatch_table,
        &code_gen.hir_types,
    );

    // Collect all used types
    for def in code_gen.db.module_data(file_id).definitions() {
        match def {
            ModuleDef::Struct(s) => {
                type_table_builder.collect_struct(*s);
            }
            ModuleDef::Function(f) => {
                type_table_builder.collect_fn(*f);
            }
            ModuleDef::BuiltinType(_) | ModuleDef::TypeAlias(_) => (),
        }
    }

    let type_table = type_table_builder.build();

    // Create the allocator handle global value
    let allocator_handle_type = if needs_alloc {
        let allocator_handle_type = code_gen.context.i8_type().ptr_type(AddressSpace::Generic);
        let global = llvm_module.add_global(allocator_handle_type, None, "allocatorHandle");
        global.set_initializer(&allocator_handle_type.const_null());
        global.set_unnamed_address(UnnamedAddress::Global);
        Some(allocator_handle_type)
    } else {
        None
    };

    FileGroupIR {
        llvm_module,
        dispatch_table,
        type_table,
        allocator_handle_type,
    }
}

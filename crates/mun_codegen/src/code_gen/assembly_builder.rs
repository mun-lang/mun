use crate::{
    assembly::Assembly,
    code_gen::{optimize_module, symbols, CodeGenContext, CodeGenerationError},
    ir::{file::gen_file_ir, file_group::gen_file_group_ir},
    value::{IrTypeContext, IrValueContext},
    ModuleGroupId, ModulePartition,
};
use inkwell::module::{Linkage, Module};
use rustc_hash::FxHashSet;

/// A struct that can be used to build an `Assembly<'db, 'ink', ctx>`
pub struct AssemblyBuilder<'db, 'ink, 'ctx, 't> {
    code_gen: &'ctx CodeGenContext<'db, 'ink>,
    module_group_partition: &'t ModulePartition,
    module_group_id: ModuleGroupId,
    assembly_module: Module<'ink>,
}

impl<'db, 'ink, 'ctx, 't> AssemblyBuilder<'db, 'ink, 'ctx, 't> {
    /// Constructs a new `AssemblyBuilder` for the given module group.
    pub(crate) fn new(
        code_gen: &'ctx CodeGenContext<'db, 'ink>,
        module_group_partition: &'t ModulePartition,
        module_group_id: ModuleGroupId,
    ) -> Result<Self, anyhow::Error> {
        // Construct a module for the assembly
        let module_group = &module_group_partition[module_group_id];
        let assembly_module = code_gen.create_module(&module_group.name);

        Ok(Self {
            code_gen,
            module_group_id,
            assembly_module,
            module_group_partition,
        })
    }

    /// Constructs an object file.
    pub fn build(self) -> Result<Assembly<'db, 'ink, 'ctx>, anyhow::Error> {
        let module_group = &self.module_group_partition[self.module_group_id];
        let group_ir = gen_file_group_ir(self.code_gen, module_group);
        let file = gen_file_ir(self.code_gen, &group_ir, module_group);

        // Clone the LLVM modules so that we can modify it without modifying the cached value.
        self.assembly_module
            .link_in_module(group_ir.llvm_module.clone())
            .map_err(|e| CodeGenerationError::ModuleLinkerError(e.to_string()))?;

        self.assembly_module
            .link_in_module(file.llvm_module.clone())
            .map_err(|e| CodeGenerationError::ModuleLinkerError(e.to_string()))?;

        if self.code_gen.db.target().options.is_like_windows {
            // Add the useless `_fltused` symbol to indicate that the object file supports
            // floating-point values. This is required for Windows.
            let fltused =
                self.assembly_module
                    .add_global(self.code_gen.context.i32_type(), None, "_fltused");
            fltused.set_initializer(&self.code_gen.context.i32_type().const_int(1, false));
            fltused.set_linkage(Linkage::External);
        }

        let target_data = self.code_gen.target_machine.get_target_data();
        let type_context = IrTypeContext {
            context: &self.code_gen.context,
            target_data: &target_data,
            struct_types: &self.code_gen.rust_types,
        };

        let value_context = IrValueContext {
            type_context: &type_context,
            context: type_context.context,
            module: &self.assembly_module,
        };

        // Build the set of dependencies
        let dependencies = group_ir
            .referenced_modules
            .iter()
            .filter_map(|&module| self.module_group_partition.group_for_module(module))
            .collect::<FxHashSet<_>>()
            .into_iter()
            .map(|group_id| {
                self.module_group_partition[group_id]
                    .relative_file_path()
                    .to_string()
            })
            .collect();

        // Generate the `get_info` method.
        symbols::gen_reflection_ir(
            self.code_gen.db,
            &value_context,
            &file.api,
            &group_ir.dispatch_table,
            &group_ir.type_table,
            &self.code_gen.hir_types,
            self.code_gen.optimization_level,
            dependencies,
        );

        // Optimize the assembly module
        optimize_module(&self.assembly_module, self.code_gen.optimization_level);

        // Debug print the IR
        //println!("{}", assembly_module.print_to_string().to_string());

        Ok(Assembly::new(self.code_gen, self.assembly_module))
    }
}

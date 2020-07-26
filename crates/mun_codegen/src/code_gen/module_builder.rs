use crate::code_gen::object_file::ObjectFile;
use crate::code_gen::{optimize_module, symbols, CodeGenContext, CodeGenerationError};
use crate::ir::file::gen_file_ir;
use crate::ir::file_group::gen_file_group_ir;
use crate::value::{IrTypeContext, IrValueContext};
use hir::FileId;
use inkwell::module::{Linkage, Module};

/// A struct that can be used to build an LLVM `Module`.
pub struct ModuleBuilder<'db, 'ink, 'ctx> {
    code_gen: &'ctx CodeGenContext<'db, 'ink>,
    file_id: FileId,
    assembly_module: Module<'ink>,
}

impl<'db, 'ink, 'ctx> ModuleBuilder<'db, 'ink, 'ctx> {
    /// Constructs a module for the given `hir::FileId` using the provided `CodeGenContext`.
    pub fn new(
        code_gen: &'ctx CodeGenContext<'db, 'ink>,
        file_id: FileId,
    ) -> Result<Self, anyhow::Error> {
        // Construct a module for the assembly
        let assembly_name = code_gen.db.file_relative_path(file_id);
        let assembly_module = code_gen.create_module(assembly_name);

        Ok(Self {
            code_gen,
            file_id,
            assembly_module,
        })
    }

    /// Constructs an object file.
    pub fn build(self) -> Result<ObjectFile, anyhow::Error> {
        let group_ir = gen_file_group_ir(self.code_gen, self.file_id);
        let file = gen_file_ir(self.code_gen, &group_ir, self.file_id);

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

        // Generate the `get_info` method.
        symbols::gen_reflection_ir(
            self.code_gen.db,
            &value_context,
            &file.api,
            &group_ir.dispatch_table,
            &group_ir.type_table,
            &self.code_gen.hir_types,
            self.code_gen.optimization_level,
        );

        // Optimize the assembly module
        optimize_module(&self.assembly_module, self.code_gen.optimization_level);

        // Debug print the IR
        //println!("{}", assembly_module.print_to_string().to_string());

        ObjectFile::new(
            &self.code_gen.db.target(),
            &self.code_gen.target_machine,
            &self.assembly_module,
        )
    }
}

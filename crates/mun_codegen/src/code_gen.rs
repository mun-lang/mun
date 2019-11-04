use crate::code_gen::linker::LinkerError;
use crate::IrDatabase;
use failure::Fail;
use inkwell::module::Module;
use inkwell::passes::{PassManager, PassManagerBuilder};
use inkwell::targets::{CodeModel, FileType, InitializationConfig, RelocMode, Target};
use inkwell::OptimizationLevel;
use mun_hir::FileId;
use std::io::{self, Write};
use std::path::Path;

mod abi_types;
mod linker;

#[derive(Debug, Fail)]
enum CodeGenerationError {
    #[fail(display = "{}", 0)]
    LinkerError(#[fail(cause)] LinkerError),
    #[fail(display = "error linking modules: {}", 0)]
    ModuleLinkerError(String),
    #[fail(display = "unknown target triple: {}", 0)]
    UnknownTargetTriple(String),
    #[fail(display = "error creating target machine")]
    CouldNotCreateTargetMachine,
    #[fail(display = "error creating object file")]
    CouldNotCreateObjectFile(io::Error),
    #[fail(display = "error generating machine code")]
    CodeGenerationError(String),
}

impl From<LinkerError> for CodeGenerationError {
    fn from(e: LinkerError) -> Self {
        CodeGenerationError::LinkerError(e)
    }
}

/// Construct a shared object for the given `hir::FileId` at the specified output file location.
pub fn write_module_shared_object(
    db: &impl IrDatabase,
    file_id: FileId,
    output_file_path: &Path,
) -> Result<(), failure::Error> {
    let target = db.target();

    // Construct a module for the assembly
    let assembly_module = db.context().create_module(
        output_file_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown"),
    );

    // Generate IR for the module and clone it so that we can modify it without modifying the
    // cached value.
    let module = db.module_ir(file_id);
    assembly_module
        .link_in_module(module.llvm_module.clone())
        .map_err(|e| CodeGenerationError::ModuleLinkerError(e.to_string()))?;

    // Generate the `get_info` method.
    symbols::gen_reflection_ir(
        db,
        &module.functions,
        &module.dispatch_table,
        &assembly_module,
    );

    // Initialize the x86 target
    Target::initialize_x86(&InitializationConfig::default());

    // Construct the LLVM target from the specified target.
    let llvm_target = Target::from_triple(&target.llvm_target)
        .map_err(|e| CodeGenerationError::UnknownTargetTriple(e.to_string()))?;
    assembly_module.set_target(&llvm_target);

    // Optimize the assembly module
    optimize_module(&assembly_module, db.optimization_lvl());

    // Debug print the IR
    //println!("{}", assembly_module.print_to_string().to_string());

    // Construct target machine for machine code generation
    let target_machine = llvm_target
        .create_target_machine(
            &target.llvm_target,
            &target.options.cpu,
            &target.options.features,
            db.optimization_lvl(),
            RelocMode::PIC,
            CodeModel::Default,
        )
        .ok_or(CodeGenerationError::CouldNotCreateTargetMachine)?;

    // Generate object file
    let obj_file = {
        let obj = target_machine
            .write_to_memory_buffer(&assembly_module, FileType::Object)
            .map_err(|e| CodeGenerationError::CodeGenerationError(e.to_string()))?;
        let mut obj_file = tempfile::NamedTempFile::new()
            .map_err(CodeGenerationError::CouldNotCreateObjectFile)?;
        obj_file
            .write(obj.as_slice())
            .map_err(CodeGenerationError::CouldNotCreateObjectFile)?;
        obj_file
    };

    // Construct a linker for the target
    let mut linker = linker::create_with_target(&target);
    linker.add_object(obj_file.path())?;

    // Link the object
    linker.build_shared_object(&output_file_path)?;
    linker.finalize()?;

    Ok(())
}

/// Optimizes the specified LLVM `Module` using the default passes for the given
/// `OptimizationLevel`.
fn optimize_module(module: &Module, optimization_lvl: OptimizationLevel) {
    let pass_builder = PassManagerBuilder::create();
    pass_builder.set_optimization_level(optimization_lvl);

    let module_pass_manager = PassManager::create(());
    pass_builder.populate_module_pass_manager(&module_pass_manager);
    module_pass_manager.run_on(module);
}

pub mod symbols;

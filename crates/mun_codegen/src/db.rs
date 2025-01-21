use std::{rc::Rc, sync::Arc};

use by_address::ByAddress;
use inkwell::targets::{CodeModel, InitializationConfig, RelocMode, Target, TargetTriple};

use crate::{AssemblyIr, ModuleGroupId, ModulePartition, TargetAssembly};

/// The `CodeGenDatabase` enables caching of code generation stages.
/// Inkwell/LLVM objects are not stored in the cache because they are not
/// thread-safe.
///
/// The main purpose of using this Salsa database is to enable caching of
/// high-level objects based on changes to source files. Although the code
/// generation cache is pretty granular there is still a benefit to not having
/// to recompile assemblies if not required.
#[salsa::query_group(CodeGenDatabaseStorage)]
pub trait CodeGenDatabase: mun_hir::HirDatabase + mun_db::Upcast<dyn mun_hir::HirDatabase> {
    /// Set the optimization level used to generate assemblies
    #[salsa::input]
    fn optimization_level(&self) -> inkwell::OptimizationLevel;

    /// Returns the current module partition
    #[salsa::invoke(crate::module_partition::build_partition)]
    fn module_partition(&self) -> Arc<ModulePartition>;
}

#[salsa::query_group(LlvmCodeGenDatabaseStorage)]
pub trait LlvmCodeGenDatabase: CodeGenDatabase {
    /// Returns the inkwell target machine that completely describes the code
    /// generation target. All target-specific information should be
    /// accessible through this interface.
    fn target_machine(&self) -> ByAddress<Rc<inkwell::targets::TargetMachine>>;

    /// Returns a file containing the IR for the specified module.
    #[salsa::invoke(crate::assembly::build_assembly_ir)]
    fn assembly_ir(&self, module_group: ModuleGroupId) -> Arc<AssemblyIr>;

    /// Returns a fully linked shared object for the specified module.
    #[salsa::invoke(crate::assembly::build_target_assembly)]
    fn target_assembly(&self, module_group: ModuleGroupId) -> Arc<TargetAssembly>;
}

/// Constructs the primary interface to the complete machine description for the
/// target machine. All target-specific information should be accessible through
/// this interface.
fn target_machine(db: &dyn LlvmCodeGenDatabase) -> ByAddress<Rc<inkwell::targets::TargetMachine>> {
    // Get the HIR target
    let target = db.target();

    // Initialize the x86 target
    Target::initialize_x86(&InitializationConfig::default());
    Target::initialize_aarch64(&InitializationConfig::default());

    // Retrieve the LLVM target using the specified target.
    let target_triple = TargetTriple::create(&db.target().llvm_target);
    let llvm_target = Target::from_triple(&target_triple)
        .expect("could not find llvm target tripple for Mun target");

    // Construct target machine for machine code generation
    let target_machine = llvm_target
        .create_target_machine(
            &target_triple,
            &target.options.cpu,
            &target.options.features,
            db.optimization_level(),
            RelocMode::PIC,
            CodeModel::Default,
        )
        .expect("could not create llvm target machine");

    ByAddress(Rc::new(target_machine))
}

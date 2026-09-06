use std::rc::Rc;

use inkwell::{context::Context, module::Module, targets::TargetMachine};

use crate::{
    ir::{ty::HirTypeCache, types::AbiTypes},
    CodeGenDatabase,
};

pub struct CodeGenContext<'db, 'ink> {
    /// The current LLVM context
    pub context: &'ink Context,

    /// The Salsa HIR database
    pub db: &'db dyn mun_hir::HirDatabase,

    /// The LLVM representation of the runtime ABI.
    pub abi_types: AbiTypes<'ink>,

    /// A mapping from HIR types to LLVM struct types
    pub hir_types: HirTypeCache<'db, 'ink>,

    /// The optimization level
    pub optimization_level: inkwell::OptimizationLevel,

    /// The target to generate code for
    pub target_machine: Rc<TargetMachine>,
}

impl<'db, 'ink> CodeGenContext<'db, 'ink> {
    /// Constructs a new `CodeGenContext` from an LLVM context and a
    /// `CodeGenDatabase`.
    pub fn new(context: &'ink Context, db: &'db dyn CodeGenDatabase) -> Self {
        let target_machine = db.target_machine().0;
        let target_data = target_machine.get_target_data();
        Self {
            context,
            abi_types: AbiTypes::new(context, &target_data),
            hir_types: HirTypeCache::new(context, db, target_data),
            optimization_level: db.optimization_level(),
            target_machine,
            db,
        }
    }

    /// Constructs a new `Module` with the specified name and initialized for
    /// the target.
    pub fn create_module(&self, name: impl AsRef<str>) -> Module<'ink> {
        let module = self.context.create_module(name.as_ref());
        module.set_data_layout(&self.target_machine.get_target_data().get_data_layout());
        module.set_triple(&self.target_machine.get_triple());
        module
    }
}

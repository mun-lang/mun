use std::{cell::RefCell, collections::HashMap, rc::Rc};

use inkwell::{context::Context, module::Module, targets::TargetMachine, types::StructType};

use crate::{ir::ty::HirTypeCache, CodeGenDatabase};

pub struct CodeGenContext<'db, 'ink> {
    /// The current LLVM context
    pub context: &'ink Context,

    /// The Salsa HIR database
    pub db: &'db dyn mun_hir::HirDatabase,

    /// A mapping from Rust types' full path (without lifetime) to inkwell types
    pub rust_types: RefCell<HashMap<&'static str, StructType<'ink>>>,

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
        Self {
            context,
            rust_types: RefCell::new(HashMap::default()),
            hir_types: HirTypeCache::new(context, db, target_machine.get_target_data()),
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

use crate::{
    code_gen::{CodeGenContext, ModuleBuilder, ObjectFile},
    db::CodeGenDatabase,
};
use anyhow::anyhow;
use inkwell::context::Context;
use std::{path::Path, sync::Arc};
use tempfile::NamedTempFile;

/// An `Assembly` is a successfully linked module of code from one or more files.
pub struct Assembly<'db, 'ink, 'ctx> {
    code_gen: &'ctx CodeGenContext<'db, 'ink>,
    module: inkwell::module::Module<'ink>,
}

impl<'db, 'ink, 'ctx> Assembly<'db, 'ink, 'ctx> {
    /// Constructs an assembly
    pub fn new(
        code_gen: &'ctx CodeGenContext<'db, 'ink>,
        module: inkwell::module::Module<'ink>,
    ) -> Self {
        Self { code_gen, module }
    }

    /// Tries to convert the assembly into an `ObjectFile`.
    pub fn into_object_file(self) -> Result<ObjectFile, anyhow::Error> {
        ObjectFile::new(
            &self.code_gen.db.target(),
            &self.code_gen.target_machine,
            &self.module,
        )
    }

    /// Tries to write the `Assembly`'s IR to file.
    pub fn write_ir_to_file(self, output_path: &Path) -> Result<(), anyhow::Error> {
        self.module
            .print_to_file(output_path)
            .map_err(|e| anyhow!("{}", e))
    }
}

/// Builds an assembly for the specified file
fn build_assembly<'db, 'ink, 'ctx>(
    code_gen_context: &'ctx CodeGenContext<'db, 'ink>,
    module: hir::Module,
) -> Assembly<'db, 'ink, 'ctx> {
    let module_builder =
        ModuleBuilder::new(code_gen_context, module).expect("could not create ModuleBuilder");

    module_builder.build().expect("unable to create assembly")
}

/// A `TargetAssembly` is a reference to a Mun library stored on disk.
#[derive(Debug)]
pub struct TargetAssembly {
    file: NamedTempFile,
}

impl PartialEq for TargetAssembly {
    fn eq(&self, other: &Self) -> bool {
        self.path().eq(other.path())
    }
}

impl Eq for TargetAssembly {}

impl TargetAssembly {
    pub const EXTENSION: &'static str = "munlib";

    /// Returns the current location of the assembly
    pub fn path(&self) -> &Path {
        self.file.path()
    }

    /// Copies the assembly to the specified location
    pub fn copy_to<P: AsRef<Path>>(&self, destination: P) -> Result<(), std::io::Error> {
        std::fs::copy(self.path(), destination).map(|_| ())
    }
}

/// Builds an assembly for the specified module.
pub(crate) fn build_target_assembly(
    db: &dyn CodeGenDatabase,
    module: hir::Module,
) -> Arc<TargetAssembly> {
    // Setup the code generation context
    let inkwell_context = Context::create();
    let code_gen_context = CodeGenContext::new(&inkwell_context, db);

    // Build an assembly for the module
    let assembly = build_assembly(&code_gen_context, module);

    // Convert the assembly into an object file
    let obj_file = assembly
        .into_object_file()
        .expect("unable to create object file");

    // Construct a temporary file for the assembly
    let file = NamedTempFile::new().expect("could not create temp file for shared object");

    // Translate the object file into a shared object
    obj_file
        .into_shared_object(file.path())
        .expect("could not link object file");

    Arc::new(TargetAssembly { file })
}

/// An `AssemblyIR` is a reference to an IR file stored on disk.
#[derive(Debug)]
pub struct AssemblyIR {
    file: NamedTempFile,
}

impl PartialEq for AssemblyIR {
    fn eq(&self, other: &Self) -> bool {
        self.path().eq(other.path())
    }
}

impl Eq for AssemblyIR {}

impl AssemblyIR {
    pub const EXTENSION: &'static str = "ll";

    /// Returns the current location of the IR File.
    pub fn path(&self) -> &Path {
        self.file.path()
    }

    /// Copies the assembly to the specified location
    pub fn copy_to<P: AsRef<Path>>(&self, destination: P) -> Result<(), std::io::Error> {
        std::fs::copy(self.path(), destination).map(|_| ())
    }
}

/// Builds an IR file for the specified module.
pub(crate) fn build_assembly_ir(db: &dyn CodeGenDatabase, module: hir::Module) -> Arc<AssemblyIR> {
    // Setup the code generation context
    let inkwell_context = Context::create();
    let code_gen_context = CodeGenContext::new(&inkwell_context, db);

    // Build an assembly for the file
    let assembly = build_assembly(&code_gen_context, module);

    // Construct a temporary file for the assembly
    let file = NamedTempFile::new().expect("could not create temp file for shared object");

    // Write the assembly's IR to disk
    assembly
        .write_ir_to_file(file.path())
        .expect("could not write to temp file");

    Arc::new(AssemblyIR { file })
}

use crate::{
    code_gen::{AssemblyBuilder, CodeGenContext, ObjectFile},
    db::CodeGenDatabase,
    ModuleGroupId,
};
use anyhow::anyhow;
use apple_codesign::{SigningSettings, UnifiedSigner};
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
    db: &'db dyn CodeGenDatabase,
    code_gen: &'ctx CodeGenContext<'db, 'ink>,
    module_group_id: ModuleGroupId,
) -> Assembly<'db, 'ink, 'ctx> {
    // Setup the code generation context
    let module_partition = db.module_partition();

    let module_builder = AssemblyBuilder::new(code_gen, &module_partition, module_group_id);
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
    module_group: ModuleGroupId,
) -> Arc<TargetAssembly> {
    // Setup the code generation context
    let inkwell_context = Context::create();
    let code_gen_context = CodeGenContext::new(&inkwell_context, db);

    // Build an assembly for the module
    let assembly = build_assembly(db, &code_gen_context, module_group);

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

    let target = db.target();
    if target.options.is_like_osx {
        let signer = UnifiedSigner::new(SigningSettings::default());
        signer
            .sign_path_in_place(file.path())
            .expect("Failed to sign shared object");
    }

    Arc::new(TargetAssembly { file })
}

/// An `AssemblyIr` is a reference to an IR file stored on disk.
#[derive(Debug)]
pub struct AssemblyIr {
    file: NamedTempFile,
}

impl PartialEq for AssemblyIr {
    fn eq(&self, other: &Self) -> bool {
        self.path().eq(other.path())
    }
}

impl Eq for AssemblyIr {}

impl AssemblyIr {
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
pub(crate) fn build_assembly_ir(
    db: &dyn CodeGenDatabase,
    module_group: ModuleGroupId,
) -> Arc<AssemblyIr> {
    // Setup the code generation context
    let inkwell_context = Context::create();
    let code_gen_context = CodeGenContext::new(&inkwell_context, db);

    // Build an assembly for the module
    let assembly = build_assembly(db, &code_gen_context, module_group);

    // Construct a temporary file for the assembly
    let file = NamedTempFile::new().expect("could not create temp file for shared object");

    // Write the assembly's IR to disk
    assembly
        .write_ir_to_file(file.path())
        .expect("could not write to temp file");

    Arc::new(AssemblyIr { file })
}

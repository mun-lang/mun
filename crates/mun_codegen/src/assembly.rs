use crate::code_gen::{CodeGenContext, ModuleBuilder};
use crate::db::CodeGenDatabase;
use inkwell::context::Context;
use std::path::Path;
use std::sync::Arc;
use tempfile::NamedTempFile;

/// An `Assembly` is a reference to a Mun library stored on disk.
#[derive(Debug)]
pub struct Assembly {
    file: NamedTempFile,
}

impl PartialEq for Assembly {
    fn eq(&self, other: &Self) -> bool {
        self.path().eq(other.path())
    }
}

impl Eq for Assembly {}

impl Assembly {
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

/// Builds an assembly for the specified file
pub(crate) fn build_assembly(db: &dyn CodeGenDatabase, file_id: hir::FileId) -> Arc<Assembly> {
    // Construct a temporary file for the assembly
    let file = NamedTempFile::new().expect("could not create temp file for shared object");

    // Setup the code generation context
    let inkwell_context = Context::create();
    let code_gen_context = CodeGenContext::new(&inkwell_context, db);

    // Construct the module
    let module_builder =
        ModuleBuilder::new(&code_gen_context, file_id).expect("could not create ModuleBuilder");
    let obj_file = module_builder
        .build()
        .expect("unable to create object file");

    // Translate the object file into a shared object
    obj_file
        .into_shared_object(file.path())
        .expect("could not link object file");

    Arc::new(Assembly { file })
}

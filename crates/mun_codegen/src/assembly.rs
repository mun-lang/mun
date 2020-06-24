use crate::{IrDatabase, ModuleBuilder};
use std::path::Path;
use std::sync::Arc;
use tempfile::NamedTempFile;

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

/// Create a new temporary file that contains the linked object
pub fn assembly_query(db: &impl IrDatabase, file_id: hir::FileId) -> Arc<Assembly> {
    let file = NamedTempFile::new().expect("could not create temp file for shared object");

    let module_builder = ModuleBuilder::new(db, file_id).expect("could not create ModuleBuilder");
    let obj_file = module_builder
        .build()
        .expect("unable to create object file");
    obj_file
        .into_shared_object(file.path())
        .expect("could not link object file");

    Arc::new(Assembly { file })
}

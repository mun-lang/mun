use crate::code_gen::CodeGenerationError;
use crate::linker;
use inkwell::targets::{FileType, TargetMachine};
use mun_target::spec;
use std::{io::Write, path::Path};
use tempfile::NamedTempFile;

pub struct ObjectFile {
    target: spec::Target,
    obj_file: NamedTempFile,
}

impl ObjectFile {
    /// Constructs a new object file from the specified `module` for `target`
    pub fn new(
        target: &spec::Target,
        target_machine: &TargetMachine,
        module: &inkwell::module::Module,
    ) -> Result<Self, anyhow::Error> {
        let obj = target_machine
            .write_to_memory_buffer(module, FileType::Object)
            .map_err(|e| CodeGenerationError::CodeGenerationError(e.to_string()))?;

        let mut obj_file = tempfile::NamedTempFile::new()
            .map_err(CodeGenerationError::CouldNotCreateObjectFile)?;
        obj_file
            .write(obj.as_slice())
            .map_err(CodeGenerationError::CouldNotCreateObjectFile)?;

        Ok(Self {
            target: target.clone(),
            obj_file,
        })
    }

    /// Links the object file into a shared object.
    pub fn into_shared_object(self, output_path: &Path) -> Result<(), anyhow::Error> {
        // Construct a linker for the target
        let mut linker = linker::create_with_target(&self.target);
        linker.add_object(self.obj_file.path())?;

        // Link the object
        linker.build_shared_object(output_path)?;
        linker.finalize()?;

        Ok(())
    }
}

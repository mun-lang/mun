use crate::code_gen::CodeGenerationError;
use crate::linker;
use mun_target::spec;
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

pub struct BitcodeFile {
    target: spec::Target,
    obj_file: NamedTempFile,
}

impl BitcodeFile {
    /// Constructs a new object file from the specified `module` for `target`
    pub fn new(
        target: &spec::Target,
        module: &inkwell::module::Module,
    ) -> Result<Self, anyhow::Error> {
        // Write the bitcode to a memory buffer
        let obj = module.write_bitcode_to_memory();

        // Open a temporary file
        let mut obj_file = tempfile::NamedTempFile::new()
            .map_err(CodeGenerationError::CouldNotCreateObjectFile)?;

        // Write the bitcode to the temporary file
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
        linker.build_shared_object(&output_path)?;
        linker.finalize()?;

        Ok(())
    }
}

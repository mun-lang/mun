use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CodeGenerationError {
    #[error("error linking modules: {0}")]
    ModuleLinkerError(String),
    #[error("error creating object file")]
    CouldNotCreateObjectFile(io::Error),
    #[error("error generating machine code")]
    MachineCodeError(String),
}

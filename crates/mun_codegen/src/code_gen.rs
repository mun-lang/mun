pub use error::CodeGenerationError;

// mod assembly_builder;
mod context;
mod error;
// mod object_file;
// pub mod symbols;

/// Defines the optimization level during compilation.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OptimizationLevel {
    None = 0,
    Less = 1,
    Default = 2,
    Aggressive = 3,
}

use crate::DisplayColor;
pub use mun_codegen::OptimizationLevel;
use mun_target::spec::Target;
use std::path::PathBuf;

/// Describes all the permanent settings that are used during compilations.
#[derive(Debug, Clone)]
pub struct Config {
    /// The target triple to compile the code for.
    pub target: Target,

    /// The optimization level to use for the IR generation.
    pub optimization_lvl: OptimizationLevel,

    /// The optional output directory to store all outputs. If no directory is specified all output
    /// is stored in a temporary directory.
    pub out_dir: Option<PathBuf>,

    /// Whether or not to use colors in terminal output
    pub display_color: DisplayColor,
}

impl Default for Config {
    fn default() -> Self {
        let target = Target::host_target();
        Config {
            // This unwrap is safe because we only compile for targets that have an implemented host
            // triple.
            target: target.unwrap(),
            optimization_lvl: OptimizationLevel::Default,
            out_dir: None,
            display_color: DisplayColor::Auto,
        }
    }
}

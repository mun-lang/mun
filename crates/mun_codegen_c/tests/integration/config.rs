pub use mun_codegen::OptimizationLevel;
use mun_target::spec::Target;

/// Describes all the permanent settings that are used during compilations.
#[derive(Clone, Debug)]
pub struct Config {
    /// The target triple to compile the code for.
    pub target: Target,

    /// The optimization level to use for the IR generation.
    pub optimization_lvl: OptimizationLevel,
}

impl Default for Config {
    fn default() -> Self {
        let target = Target::host_target();

        Config {
            // This unwrap is safe because we only compile for targets that have an implemented host
            // triple.
            target: target.unwrap(),
            optimization_lvl: OptimizationLevel::Default,
        }
    }
}

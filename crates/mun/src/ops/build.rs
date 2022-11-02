use std::env;
use std::path::{Path, PathBuf};

use anyhow::anyhow;
use mun_compiler::{Config, DisplayColor, Target};
use mun_project::MANIFEST_FILENAME;

use crate::ExitStatus;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum UseColor {
    Disable,
    Enable,
    Auto,
}

#[derive(clap::Args)]
pub struct Args {
    /// Path to the manifest of the project
    #[clap(long)]
    manifest_path: Option<PathBuf>,

    /// Optimization level [0,3]
    #[clap(long, short = 'O', default_value_t = 2)]
    opt_level: u8,

    /// Use color in output
    #[clap(long, value_enum)]
    color: Option<UseColor>,

    /// Emits IR instead of a *.munlib
    #[clap(long)]
    emit_ir: bool,

    /// Run the compiler in watch mode. Watch input files and trigger recompilation on changes.
    #[clap(long)]
    watch: bool,

    /// Target for machine code
    #[clap(long, value_parser=parse_target_triple)]
    target: Option<Target>,
}

fn parse_target_triple(target_triple: &str) -> Result<Target, String> {
    Target::search(target_triple)
        .ok_or_else(|| format!("could not find target for '{}'", target_triple))
}

/// This method is invoked when the executable is run with the `build` argument indicating that a
/// user requested us to build a project in the current directory or one of its parent directories.
pub fn build(args: Args) -> Result<ExitStatus, anyhow::Error> {
    log::trace!("starting build");

    let optimization_lvl = match args.opt_level {
        0 => mun_compiler::OptimizationLevel::None,
        1 => mun_compiler::OptimizationLevel::Less,
        2 => mun_compiler::OptimizationLevel::Default,
        3 => mun_compiler::OptimizationLevel::Aggressive,
        _ => return Err(anyhow!("Only optimization levels 0-3 are supported")),
    };

    let display_colors = args
        .color
        .map(|clr| match clr {
            UseColor::Disable => DisplayColor::Disable,
            UseColor::Enable => DisplayColor::Enable,
            UseColor::Auto => DisplayColor::Auto,
        })
        .or_else(|| {
            env::var("MUN_TERMINAL_COLOR")
                .map(|value| match value.as_str() {
                    "disable" => DisplayColor::Disable,
                    "enable" => DisplayColor::Enable,
                    _ => DisplayColor::Auto,
                })
                .ok()
        })
        .unwrap_or(DisplayColor::Auto);

    // Locate the manifest
    let manifest_path = match &args.manifest_path {
        None => {
            let current_dir =
                std::env::current_dir().expect("could not determine current working directory");
            find_manifest(&current_dir).ok_or_else(|| {
                anyhow::anyhow!(
                    "could not find {} in '{}' or a parent directory",
                    MANIFEST_FILENAME,
                    current_dir.display()
                )
            })?
        }
        Some(path) => std::fs::canonicalize(Path::new(&path)).map_err(|_| {
            anyhow::anyhow!(
                "'{}' does not refer to a valid manifest path",
                path.display()
            )
        })?,
    };

    log::info!("located build manifest at: {}", manifest_path.display());

    let compiler_options = Config {
        target: args
            .target
            .unwrap_or_else(|| Target::host_target().expect("unable to determine host target")),
        optimization_lvl,
        out_dir: None,
        emit_ir: args.emit_ir,
    };

    if args.watch {
        mun_compiler_daemon::compile_and_watch_manifest(
            &manifest_path,
            compiler_options,
            display_colors,
        )
    } else {
        mun_compiler::compile_manifest(&manifest_path, compiler_options, display_colors)
    }
    .map(Into::into)
}

/// Find a Mun manifest file in the specified directory or one of its parents.
fn find_manifest(directory: &Path) -> Option<PathBuf> {
    let mut current_dir = Some(directory);
    while let Some(dir) = current_dir {
        let manifest_path = dir.join(MANIFEST_FILENAME);
        if manifest_path.exists() {
            return Some(manifest_path);
        }
        current_dir = dir.parent();
    }
    None
}

#[cfg(test)]
mod test {
    use super::find_manifest;
    use mun_project::MANIFEST_FILENAME;

    #[test]
    fn test_find_manifest() {
        let dir = tempfile::Builder::new()
            .prefix("test_find_manifest")
            .tempdir()
            .unwrap();
        let path = dir.path();
        let manifest_path = path.join(MANIFEST_FILENAME);

        assert_eq!(find_manifest(path), None);

        std::fs::write(&manifest_path, "").unwrap();
        assert_eq!(find_manifest(path).as_ref(), Some(&manifest_path));

        let subdir_path = path.join("some/random/subdir");
        std::fs::create_dir_all(&subdir_path).unwrap();
        assert_eq!(find_manifest(&subdir_path).as_ref(), Some(&manifest_path));
    }
}

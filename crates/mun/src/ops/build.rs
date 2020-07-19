use std::env;
use std::path::{Path, PathBuf};

use anyhow::anyhow;
use clap::ArgMatches;
use mun_compiler::{Config, DisplayColor, Target};
use mun_project::MANIFEST_FILENAME;

use crate::ExitStatus;

/// This method is invoked when the executable is run with the `build` argument indicating that a
/// user requested us to build a project in the current directory or one of its parent directories.
pub fn build(matches: &ArgMatches) -> Result<ExitStatus, anyhow::Error> {
    log::trace!("starting build");

    let options = compiler_options(matches)?;

    // Locate the manifest
    let manifest_path = match matches.value_of("manifest-path") {
        None => {
            let current_dir =
                std::env::current_dir().expect("could not determine currrent working directory");
            find_manifest(&current_dir).ok_or_else(|| {
                anyhow::anyhow!(
                    "could not find {} in '{}' or a parent directory",
                    MANIFEST_FILENAME,
                    current_dir.display()
                )
            })?
        }
        Some(path) => std::fs::canonicalize(Path::new(path))
            .map_err(|_| anyhow::anyhow!("'{}' does not refer to a valid manifest path", path))?,
    };

    log::info!("located build manifest at: {}", manifest_path.display());

    if matches.is_present("watch") {
        mun_compiler_daemon::compile_and_watch_manifest(&manifest_path, options)
    } else {
        mun_compiler::compile_manifest(&manifest_path, options)
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

fn compiler_options(matches: &ArgMatches) -> Result<mun_compiler::Config, anyhow::Error> {
    let optimization_lvl = match matches.value_of("opt-level") {
        Some("0") => mun_compiler::OptimizationLevel::None,
        Some("1") => mun_compiler::OptimizationLevel::Less,
        None | Some("2") => mun_compiler::OptimizationLevel::Default,
        Some("3") => mun_compiler::OptimizationLevel::Aggressive,
        _ => return Err(anyhow!("Only optimization levels 0-3 are supported")),
    };

    let display_color = matches
        .value_of("color")
        .map(ToOwned::to_owned)
        .or_else(|| env::var("MUN_TERMINAL_COLOR").ok())
        .map(|value| match value.as_str() {
            "disable" => DisplayColor::Disable,
            "enable" => DisplayColor::Enable,
            _ => DisplayColor::Auto,
        })
        .unwrap_or(DisplayColor::Auto);

    Ok(Config {
        target: matches
            .value_of("target")
            .map_or_else(Target::host_target, Target::search)?,
        optimization_lvl,
        out_dir: None,
        display_color,
    })
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

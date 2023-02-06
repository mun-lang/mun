use std::path::Path;
use std::{fs, path::PathBuf};

use anyhow::anyhow;

use crate::ExitStatus;

#[derive(clap::Args)]
pub struct Args {
    path: Option<PathBuf>,
}

/// This method is invoked when the executable is run with the `init` argument indicating that a
/// user requested us to create a new project in the current directory.
pub fn init(args: Args) -> Result<ExitStatus, anyhow::Error> {
    let create_in = args.path.unwrap_or_else(|| {
        std::env::current_dir().expect("could not determine current working directory")
    });

    let project_name = create_in
        .file_name()
        .expect("Failed to fetch name of current folder.")
        .to_str()
        .expect("Project name must be valid UTF-8");

    create_project(&create_in, project_name)
}

/// This is used by `init` and `new` arguments to create projects in different paths.
pub fn create_project(create_in: &Path, project_name: &str) -> Result<ExitStatus, anyhow::Error> {
    log::trace!("Creating new project");
    {
        let manifest_path = create_in.join("mun.toml");

        write(
            manifest_path,
            format!(
                // @TODO. Nothing is done yet to find out who the author is.
                r#"[package]
name="{project_name}"
authors=[]
version="0.1.0"
"#,
            ),
        )?;
    }
    {
        let src_path = create_in.join("src");
        create_dir(&src_path)?;

        let main_file_path = src_path.join("mod.mun");

        write(
            main_file_path,
            r#"pub fn main() -> f64 {
    3.14159
}
"#,
        )?;
    }
    println!("Created `{project_name}` package");
    Ok(ExitStatus::Success)
}

/// Shortcut function for creating new directories.
pub fn create_dir(path: impl AsRef<Path>) -> anyhow::Result<()> {
    fs::create_dir(&path)
        .map_err(|_| anyhow!("failed to create directory `{}`", path.as_ref().display()))
}

/// Shortcut function for creating new files.
pub fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> anyhow::Result<()> {
    let path = path.as_ref();
    fs::write(path, contents.as_ref()).map_err(|_| anyhow!("failed to write `{}`", path.display()))
}

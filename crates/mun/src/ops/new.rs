use crate::ops::init::{create_dir, create_project};
use crate::ExitStatus;
use clap::ArgMatches;
use std::path::PathBuf;

/// This method is invoked when the executable is run with the `new` argument indicating that a
/// user requested us to create a new project in a new directory.
pub fn new(matches: &ArgMatches) -> Result<ExitStatus, anyhow::Error> {
    let create_in: PathBuf = matches
        .value_of("path")
        .expect(
            "Path argument not found: This should be unreachable as clap requires this argument.",
        )
        .into();

    let project_name = create_in
        .file_name()
        .expect("Invalid path argument.")
        .to_str()
        .expect("Project name is invalid UTF-8.");

    if create_in.exists() {
        eprint!(
            "destination `{}` already exists\n\n\
             Use `mun init` to initialize the directory",
            create_in.display()
        );
        return Ok(ExitStatus::Error);
    }
    create_dir(&create_in)?;
    create_project(&create_in, project_name)
}

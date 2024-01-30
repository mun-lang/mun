use std::path::PathBuf;

use crate::{
    ops::init::{create_dir, create_project},
    ExitStatus,
};

#[derive(clap::Args)]
pub struct Args {
    path: PathBuf,
}

/// This method is invoked when the executable is run with the `new` argument
/// indicating that a user requested us to create a new project in a new
/// directory.
pub fn new(args: Args) -> Result<ExitStatus, anyhow::Error> {
    let project_name = args
        .path
        .file_name()
        .expect("Invalid path argument.")
        .to_str()
        .expect("Project name is invalid UTF-8.");

    if args.path.exists() {
        eprint!(
            "destination `{}` already exists\n\n\
             Use `mun init` to initialize the directory",
            args.path.display()
        );
        return Ok(ExitStatus::Error);
    }
    create_dir(&args.path)?;
    create_project(&args.path, project_name)
}

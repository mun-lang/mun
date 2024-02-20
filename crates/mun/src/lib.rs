mod ops;

use std::ffi::OsString;

use clap::{Parser, Subcommand};
use ops::{build, init, language_server, new, start};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)]
enum Command {
    /// Run the Mun language server
    LanguageServer(language_server::Args),

    /// Compiles a local Mun file into a module
    Build(build::Args),

    /// Create a new Mun project at the specified location
    New(new::Args),

    /// Initialize a new Mun project in the specified location
    Init(init::Args),

    /// Invoke a function from a munlib
    Start(start::Args),
}

#[derive(Copy, Debug, Clone, PartialEq, Eq)]
pub enum ExitStatus {
    Success,
    Error,
}

impl From<bool> for ExitStatus {
    fn from(value: bool) -> Self {
        if value {
            ExitStatus::Success
        } else {
            ExitStatus::Error
        }
    }
}

pub fn run_with_args<T, I>(args: I) -> Result<ExitStatus, anyhow::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args = Args::parse_from(args);
    match args.command {
        Command::Build(args) => build::build(args),
        Command::LanguageServer(args) => language_server::language_server(args),
        Command::New(args) => new::new(args),
        Command::Init(args) => init::init(args),
        Command::Start(args) => start::start(args),
    }
}

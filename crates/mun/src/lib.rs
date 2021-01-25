mod ops;

use std::env;
use std::ffi::OsString;

use clap::{App, AppSettings, Arg, SubCommand};
use mun_project::MANIFEST_FILENAME;

use ops::{build, init, language_server, new, start};

#[derive(Copy, Debug, Clone, PartialEq, Eq)]
pub enum ExitStatus {
    Success,
    Error,
}

impl Into<ExitStatus> for bool {
    fn into(self) -> ExitStatus {
        if self {
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
    let matches = App::new("mun")
        .version(env!("CARGO_PKG_VERSION"))
        .author("The Mun Project Developers")
        .about("The Mun executable enables compiling and running standalone Mun code")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("build")
                .arg(
                    Arg::with_name("manifest-path")
                        .long("manifest-path")
                        .takes_value(true)
                        .help(&format!("Path to {}", MANIFEST_FILENAME)),
                )
                .arg(Arg::with_name("watch").long("watch").help(
                    "Run the compiler in watch mode.\
                        Watch input files and trigger recompilation on changes.",
                ))
                .arg(
                    Arg::with_name("opt-level")
                        .short("O")
                        .long("opt-level")
                        .takes_value(true)
                        .help("optimize with possible levels 0-3"),
                )
                .arg(
                    Arg::with_name("target")
                        .long("target")
                        .takes_value(true)
                        .help("target triple for which code is compiled"),
                )
                .arg(
                    Arg::with_name("color")
                        .long("color")
                        .takes_value(true)
                        .possible_values(&["enable", "auto", "disable"])
                        .help("color text in terminal"),
                )
                .arg(
                    Arg::with_name("emit-ir")
                        .long("emit-ir")
                        .help("emits IR instead of a *.munlib"),
                )
                .about("Compiles a local Mun file into a module"),
        )
        .subcommand(
            SubCommand::with_name("start")
                .arg(
                    Arg::with_name("LIBRARY")
                        .help("Sets the library to use")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::with_name("entry")
                        .long("entry")
                        .takes_value(true)
                        .help("the function entry point to call on startup"),
                ),
        )
        .subcommand(
            SubCommand::with_name("new").arg(
                Arg::with_name("path")
                    .help("the path to create a new project")
                    .required(true)
                    .index(1),
            ),
        )
        .subcommand(
            SubCommand::with_name("init").arg(
                Arg::with_name("path")
                    .help("the path to create a new project [default: .]")
                    .index(1),
            ),
        )
        .subcommand(SubCommand::with_name("language-server"))
        .get_matches_from_safe(args);

    match matches {
        Ok(matches) => match matches.subcommand() {
            ("build", Some(matches)) => build(matches),
            ("language-server", Some(matches)) => language_server(matches),
            ("start", Some(matches)) => start(matches).map(|_| ExitStatus::Success),
            ("new", Some(matches)) => new(matches),
            ("init", Some(matches)) => init(matches),
            _ => unreachable!(),
        },
        Err(e) => {
            eprint!("{}", e.message);
            Ok(ExitStatus::Error)
        }
    }
}

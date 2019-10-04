#[macro_use]
extern crate failure;

use std::time::Duration;

use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use mun_runtime::invoke_fn;

fn main() -> Result<(), failure::Error> {
    let matches = App::new("mun")
        .version(env!("CARGO_PKG_VERSION"))
        .author("The Mun Project Developers")
        .about("The Mun executable enables compiling and running standalone Mun code")
        .setting(AppSettings::SubcommandRequired)
        .subcommand(
            SubCommand::with_name("build")
                .arg(
                    Arg::with_name("INPUT")
                        .help("Sets the input file to use")
                        .required(true)
                        .index(1),
                )
                .arg(Arg::with_name("watch").long("watch").help(
                    "Run the compiler in watch mode.
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
                )
                .arg(
                    Arg::with_name("delay")
                        .long("delay")
                        .required(true)
                        .takes_value(true)
                        .help("how much to delay received filesystem events (in ms). This allows bundling of identical events, e.g. when several writes to the same file are detected. A high delay will make hot reloading less responsive."),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        ("build", Some(matches)) => build(matches)?,
        ("start", Some(matches)) => start(matches)?,
        _ => unreachable!(),
    }

    Ok(())
}

/// Build the source file specified
fn build(matches: &ArgMatches) -> Result<(), failure::Error> {
    let options = compiler_options(matches)?;
    if matches.is_present("watch") {
        mun_compiler_daemon::main(&options)
    } else {
        mun_compiler::main(&options)
    }
}

/// Starts the runtime with the specified library and invokes function `entry`.
fn start(matches: &ArgMatches) -> Result<(), failure::Error> {
    let runtime_options = runtime_options(matches)?;
    let mut runtime = mun_runtime::MunRuntime::new(runtime_options)?;

    let entry_point = matches.value_of("entry").unwrap_or("main");
    Ok(invoke_fn!(runtime, entry_point))
}

fn compiler_options(matches: &ArgMatches) -> Result<mun_compiler::CompilerOptions, failure::Error> {
    let optimization_lvl = match matches.value_of("opt-level") {
        Some("0") => mun_compiler::OptimizationLevel::None,
        Some("1") => mun_compiler::OptimizationLevel::Less,
        None | Some("2") => mun_compiler::OptimizationLevel::Default,
        Some("3") => mun_compiler::OptimizationLevel::Aggressive,
        _ => return Err(format_err!("Only optimization levels 0-3 are supported")),
    };

    Ok(mun_compiler::CompilerOptions {
        input: matches.value_of("INPUT").unwrap().into(), // Safe because its a required arg
        target: matches.value_of("target").map(|t| t.to_string()),
        optimization_lvl,
    })
}

fn runtime_options(matches: &ArgMatches) -> Result<mun_runtime::RuntimeOptions, failure::Error> {
    let delay: u64 = matches.value_of("delay").unwrap().parse()?;

    Ok(mun_runtime::RuntimeOptions {
        library_path: matches.value_of("LIBRARY").unwrap().into(), // Safe because its a required arg
        delay: Duration::from_millis(delay),
    })
}

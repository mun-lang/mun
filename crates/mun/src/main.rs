#[macro_use]
extern crate failure;

use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};

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
        .get_matches();

    match matches.subcommand() {
        ("build", Some(matches)) => {
            let optimization_lvl = match matches.value_of("opt-level") {
                Some("0") => mun_compiler::OptimizationLevel::None,
                Some("1") => mun_compiler::OptimizationLevel::Less,
                None | Some("2") => mun_compiler::OptimizationLevel::Default,
                Some("3") => mun_compiler::OptimizationLevel::Aggressive,
                _ => return Err(format_err!("Only optimization levels 0-3 are supported")),
            };

            let target = matches.value_of("target").map(|t| t.to_string());

            build(matches, optimization_lvl, target)?
        }
        _ => unreachable!(),
    }

    Ok(())
}

/// Build the source file specified
fn build(
    matches: &ArgMatches,
    optimization_lvl: mun_compiler::OptimizationLevel,
    target: Option<String>,
) -> Result<(), failure::Error> {
    let options = mun_compiler::CompilerOptions {
        input: matches.value_of("INPUT").unwrap().into(), // Safe because its a required arg
        target,
        optimization_lvl,
    };
    mun_compiler::main(options)
}

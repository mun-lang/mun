#[macro_use]
extern crate failure;

use std::cell::RefCell;
use std::env;
use std::rc::Rc;
use std::time::Duration;

use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use mun_compiler::{Config, DisplayColor, PathOrInline, Target};
use mun_runtime::{invoke_fn, ReturnTypeReflection, Runtime, RuntimeBuilder};

fn main() -> Result<(), failure::Error> {
    let matches = App::new("mun")
        .version(env!("CARGO_PKG_VERSION"))
        .author("The Mun Project Developers")
        .about("The Mun executable enables compiling and running standalone Mun code")
        .setting(AppSettings::SubcommandRequiredElseHelp)
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
                .arg(
                    Arg::with_name("color")
                        .long("color")
                        .takes_value(true)
                        .possible_values(&["enable", "auto", "disable"])
                        .help("color text in terminal"),
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
                        .takes_value(true)
                        .help("how much to delay received filesystem events (in ms). This allows bundling of identical events, e.g. when several writes to the same file are detected. A high delay will make hot reloading less responsive. (defaults to 10 ms)"),
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
        mun_compiler_daemon::main(options)
    } else {
        mun_compiler::main(options).map(|_| {})
    }
}

/// Starts the runtime with the specified library and invokes function `entry`.
fn start(matches: &ArgMatches) -> Result<(), failure::Error> {
    let runtime = Rc::new(RefCell::new(runtime(matches)?));

    let borrowed = runtime.borrow();
    let entry_point = matches.value_of("entry").unwrap_or("main");
    let fn_info = borrowed.get_function_info(entry_point).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Failed to obtain entry point '{}'", entry_point),
        )
    })?;

    if let Some(ret_type) = fn_info.signature.return_type() {
        let type_guid = &ret_type.guid;
        if *type_guid == bool::type_guid() {
            let result: bool =
                invoke_fn!(runtime, entry_point).map_err(|e| failure::err_msg(format!("{}", e)))?;

            println!("{}", result)
        } else if *type_guid == f64::type_guid() {
            let result: f64 =
                invoke_fn!(runtime, entry_point).map_err(|e| failure::err_msg(format!("{}", e)))?;

            println!("{}", result)
        } else if *type_guid == i64::type_guid() {
            let result: i64 =
                invoke_fn!(runtime, entry_point).map_err(|e| failure::err_msg(format!("{}", e)))?;

            println!("{}", result)
        } else {
            return Err(failure::err_msg(format!(
                "Only native Mun return types are supported for entry points. Found: {}",
                ret_type.name()
            )));
        };
        Ok(())
    } else {
        #[allow(clippy::unit_arg)]
        invoke_fn!(runtime, entry_point).map_err(|e| failure::err_msg(format!("{}", e)))
    }
}

fn compiler_options(matches: &ArgMatches) -> Result<mun_compiler::CompilerOptions, failure::Error> {
    let optimization_lvl = match matches.value_of("opt-level") {
        Some("0") => mun_compiler::OptimizationLevel::None,
        Some("1") => mun_compiler::OptimizationLevel::Less,
        None | Some("2") => mun_compiler::OptimizationLevel::Default,
        Some("3") => mun_compiler::OptimizationLevel::Aggressive,
        _ => return Err(format_err!("Only optimization levels 0-3 are supported")),
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

    Ok(mun_compiler::CompilerOptions {
        input: PathOrInline::Path(matches.value_of("INPUT").unwrap().into()), // Safe because its a required arg
        config: Config {
            target: matches
                .value_of("target")
                .map_or_else(Target::host_target, Target::search)?,
            optimization_lvl,
            out_dir: None,
            display_color,
        },
    })
}

fn runtime(matches: &ArgMatches) -> Result<Runtime, failure::Error> {
    let mut builder = RuntimeBuilder::new(
        matches.value_of("LIBRARY").unwrap(), // Safe because its a required arg
    );

    if let Some(delay) = matches.value_of("delay") {
        let delay: u64 = delay.parse()?;
        builder.set_delay(Duration::from_millis(delay));
    }

    builder.spawn()
}

use clap::{App, SubCommand};

use tools::Overwrite;

fn main() -> Result<(), anyhow::Error> {
    let matches = App::new("tasks")
        .setting(clap::AppSettings::SubcommandRequiredElseHelp)
        .subcommand(SubCommand::with_name("gen-syntax"))
        .subcommand(SubCommand::with_name("gen-runtime-capi"))
        .subcommand(SubCommand::with_name("gen-abi"))
        .get_matches();
    match matches
        .subcommand_name()
        .expect("Subcommand must be specified")
    {
        "gen-syntax" => tools::syntax::generate(Overwrite)?,
        "gen-abi" => tools::abi::generate(Overwrite)?,
        "gen-runtime-capi" => tools::runtime_capi::generate(Overwrite)?,
        _ => unreachable!(),
    }
    Ok(())
}

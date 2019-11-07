use clap::{App, SubCommand};

use tools::{generate_all, Overwrite, Result};

fn main() -> Result<()> {
    let matches = App::new("tasks")
        .setting(clap::AppSettings::SubcommandRequiredElseHelp)
        .subcommand(SubCommand::with_name("gen-syntax"))
        .get_matches();
    match matches
        .subcommand_name()
        .expect("Subcommand must be specified")
    {
        "gen-syntax" => generate_all(Overwrite)?,
        _ => unreachable!(),
    }
    Ok(())
}

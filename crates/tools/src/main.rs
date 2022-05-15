use clap::{Parser, Subcommand};

use tools::{Overwrite, Result};

#[derive(Parser)]
#[clap(name = "tasks", version, author)]
struct Args {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
#[allow(clippy::enum_variant_names)]
enum Commands {
    /// Generate Rust syntax files
    GenSyntax,

    /// Generate the Mun runtime C API headers
    GenRuntimeCapi,

    /// Generate the Mun ABI headers
    GenAbi,
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Commands::GenSyntax => tools::syntax::generate(Overwrite)?,
        Commands::GenAbi => tools::abi::generate(Overwrite)?,
        Commands::GenRuntimeCapi => tools::runtime_capi::generate(Overwrite)?,
    }
    Ok(())
}

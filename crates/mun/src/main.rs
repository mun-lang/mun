use mun::{run_with_args, ExitStatus};

/// Main entry point for the `mun` executable.
fn main() -> Result<(), anyhow::Error> {
    pretty_env_logger::try_init()?;
    let status = run_with_args(std::env::args_os()).unwrap();
    match status {
        ExitStatus::Success => {}
        ExitStatus::Error => std::process::exit(1),
    };
    Ok(())
}


use clap::ArgMatches;

use crate::ExitStatus;

/// This function is invoked when the executable is invoked with the `language-server` argument. A
/// Mun language server is started ready to serve language information about one or more projects.
pub fn language_server(_matches: &ArgMatches) -> Result<ExitStatus, anyhow::Error> {
    mun_language_server::run_server().map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(ExitStatus::Success)
}

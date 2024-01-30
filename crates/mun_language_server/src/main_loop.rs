use lsp_server::Connection;

use crate::{Config, LanguageServerState};

/// Runs the main loop of the language server. This will receive requests and
/// handle them.
pub fn main_loop(connection: Connection, config: Config) -> anyhow::Result<()> {
    log::info!("initial config: {:#?}", config);
    LanguageServerState::new(connection.sender, config).run(connection.receiver)
}

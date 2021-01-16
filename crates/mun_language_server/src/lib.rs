use std::convert::TryFrom;

use serde::{de::DeserializeOwned, Serialize};

pub use config::{Config, FilesWatcher};
pub use main_loop::main_loop;
use paths::AbsPathBuf;
use project::ProjectManifest;
pub(crate) use state::LanguageServerState;

mod analysis;
mod cancelation;
mod capabilities;
mod change;
mod config;
mod conversion;
mod db;
mod diagnostics;
mod main_loop;
mod state;

/// Deserializes a `T` from a json value.
pub fn from_json<T: DeserializeOwned>(
    what: &'static str,
    json: serde_json::Value,
) -> anyhow::Result<T> {
    T::deserialize(&json)
        .map_err(|e| anyhow::anyhow!("could not deserialize {}: {}: {}", what, e, json))
}

/// Converts the `T` to a json value
pub fn to_json<T: Serialize>(value: T) -> anyhow::Result<serde_json::Value> {
    serde_json::to_value(value).map_err(|e| anyhow::anyhow!("could not serialize to json: {}", e))
}

/// Main entry point for the language server
pub fn run_server() -> anyhow::Result<()> {
    log::info!("language server started");

    // Setup IO connections
    let (connection, io_threads) = lsp_server::Connection::stdio();

    // Wait for a client to connect
    let (initialize_id, initialize_params) = connection.initialize_start()?;

    let initialize_params =
        from_json::<lsp_types::InitializeParams>("InitializeParams", initialize_params)?;

    let server_capabilities = capabilities::server_capabilities(&initialize_params.capabilities);

    let initialize_result = lsp_types::InitializeResult {
        capabilities: server_capabilities,
        server_info: Some(lsp_types::ServerInfo {
            name: String::from("mun-language-server"),
            version: Some(String::from(env!("CARGO_PKG_VERSION"))),
        }),
    };

    let initialize_result = serde_json::to_value(initialize_result).unwrap();

    connection.initialize_finish(initialize_id, initialize_result)?;

    if let Some(client_info) = initialize_params.client_info {
        log::info!(
            "client '{}' {}",
            client_info.name,
            client_info.version.unwrap_or_default()
        );
    }

    let config = {
        // Convert the root uri to a PathBuf
        let root_dir = match initialize_params
            .root_uri
            .and_then(|it| it.to_file_path().ok())
            .and_then(|path| AbsPathBuf::try_from(path).ok())
        {
            Some(path) => path,
            None => {
                // Get the current working directory as fallback
                let cwd = std::env::current_dir()?;
                AbsPathBuf::try_from(cwd)
                    .expect("could not convert current directory to an absolute path")
            }
        };

        let mut config = Config::new(root_dir);

        // Determine type of watcher to use
        let supports_file_watcher_dynamic_registration = initialize_params
            .capabilities
            .workspace
            .and_then(|c| c.did_change_watched_files)
            .map(|c| c.dynamic_registration.unwrap_or(false))
            .unwrap_or(false);
        if supports_file_watcher_dynamic_registration {
            config.watcher = FilesWatcher::Client;
        }

        // Convert the workspace_roots, if these are empy use the root_uri or the cwd
        let workspace_roots = initialize_params
            .workspace_folders
            .map(|workspaces| {
                workspaces
                    .into_iter()
                    .filter_map(|it| it.uri.to_file_path().ok())
                    .filter_map(|path| AbsPathBuf::try_from(path).ok())
                    .collect::<Vec<_>>()
            })
            .filter(|workspaces| !workspaces.is_empty())
            .unwrap_or_else(|| vec![config.root_dir.clone()]);

        // Find all the projects in the workspace
        let discovered = ProjectManifest::discover_all(workspace_roots.iter().cloned());
        log::info!("discovered projects: {:?}", discovered);
        if discovered.is_empty() {
            log::error!("failed to find any projects in {:?}", workspace_roots);
        }
        config.discovered_projects = Some(discovered);

        config
    };

    main_loop(connection, config)?;

    io_threads.join()?;
    Ok(())
}

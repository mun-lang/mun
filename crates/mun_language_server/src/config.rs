use std::path::PathBuf;

/// The configuration used by the language server.
#[derive(Debug, Clone)]
pub struct Config {
    pub watcher: FilesWatcher,
    pub workspace_roots: Vec<PathBuf>
}

impl Default for Config {
    fn default() -> Self {
        Self {
            watcher: FilesWatcher::Notify,
            workspace_roots: Vec::new()
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FilesWatcher {
    Client,
    Notify,
}

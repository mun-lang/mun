use crate::project_manifest::ProjectManifest;
use paths::AbsPathBuf;

/// The configuration used by the language server.
#[derive(Debug, Clone)]
pub struct Config {
    pub watcher: FilesWatcher,

    /// The root directory of the workspace
    pub root_dir: AbsPathBuf,

    /// A collection of projects discovered within the workspace
    pub discovered_projects: Option<Vec<ProjectManifest>>,
}

impl Config {
    /// Constructs a new instance of a `Config`
    pub fn new(root_path: AbsPathBuf) -> Self {
        Self {
            watcher: FilesWatcher::Notify,
            root_dir: root_path,
            discovered_projects: None,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum FilesWatcher {
    Client,
    Notify,
}
